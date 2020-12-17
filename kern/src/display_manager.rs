use core::time::Duration;

use dsx::sync::mutex::LockableMutex;

use mini_alloc::MiniBox;

use crate::display::{Color, ColorMode, DisplayConfig, DisplayHolder, Painter};
use crate::mbox::with_mbox;
use crate::mini_allocators::NOCACHE_ALLOC;
use crate::mutex::Mutex;
use crate::param::PAGE_SIZE;
use crate::process::KernProcessCtx;
use crate::VMM;

fn display_init() -> Option<DisplayConfig> {
    let mut width: usize = 0;
    let mut height: usize = 0;
    let mut pitch: usize = 0;
    let mut isrgb: u32 = 0;
    let mut lfb: &mut [u32] = &mut [];

    kernel_api::syscall::sleep(Duration::from_millis(500));

    with_mbox(|mbox| {
        use pi::common::*;

        mbox.0[0] = 35 * 4;
        mbox.0[1] = MBOX_REQUEST;

        mbox.0[2] = 0x48003;  //set phy wh
        mbox.0[3] = 8;
        mbox.0[4] = 8;
        mbox.0[5] = 1024;         //FrameBufferInfo.width
        mbox.0[6] = 768;          //FrameBufferInfo.height

        mbox.0[7] = 0x48004;  //set virt wh
        mbox.0[8] = 8;
        mbox.0[9] = 8;
        mbox.0[10] = 1024;        //FrameBufferInfo.virtual_width
        mbox.0[11] = 768;         //FrameBufferInfo.virtual_height

        mbox.0[12] = 0x48009; //set virt offset
        mbox.0[13] = 8;
        mbox.0[14] = 8;
        mbox.0[15] = 0;           //FrameBufferInfo.x_offset
        mbox.0[16] = 0;           //FrameBufferInfo.y.offset

        mbox.0[17] = 0x48005; //set depth
        mbox.0[18] = 4;
        mbox.0[19] = 4;
        mbox.0[20] = 32;          //FrameBufferInfo.depth

        mbox.0[21] = 0x48006; //set pixel order
        mbox.0[22] = 4;
        mbox.0[23] = 4;
        mbox.0[24] = 1;           //RGB, not BGR preferably

        mbox.0[25] = 0x40001; //get framebuffer, gets alignment on request
        mbox.0[26] = 8;
        mbox.0[27] = 8;
        mbox.0[28] = 4096 * 16;        //FrameBufferInfo.pointer
        mbox.0[29] = 0;           //FrameBufferInfo.size

        mbox.0[30] = 0x40008; //get pitch
        mbox.0[31] = 4;
        mbox.0[32] = 4;
        mbox.0[33] = 0;           //FrameBufferInfo.pitch

        mbox.0[34] = MBOX_TAG_LAST;

        if unsafe { mbox.call(MBOX_CH_PROP) } && mbox.0[20] == 32 && mbox.0[28] != 0 {
            mbox.0[28] &= 0x3FFFFFFF;    //convert GPU address to ARM address
            width = mbox.0[5] as usize;  //get actual physical width
            height = mbox.0[6] as usize; //get actual physical height
            pitch = mbox.0[33] as usize; //get number of bytes per line
            isrgb = mbox.0[24];          //get the actual channel order
            lfb = unsafe { core::slice::from_raw_parts_mut(mbox.0[28] as *mut u32, mbox.0[29] as usize / 4) };
        } else {
            error!("Unable to set screen resolution to 1024x768x32");
        }
    });

    if width == 0 {
        return None;
    }

    info!("Frame buffer: 0x{:x}, width={}, height={}, pitch={}, isrgb={}, len={}", lfb.as_ptr() as usize, width, height, pitch, isrgb, lfb.len());

    for i in (0..(lfb.len() * 4)).step_by(PAGE_SIZE) {
        let addr = (lfb.as_ptr() as usize) + i;
        unsafe { VMM.mark_page_non_cached(addr) };
    }

    // QEMU misreports color mode so hard code  BGR for now.
    let mode = if isrgb != 0 { ColorMode::RGB } else { ColorMode::BGR };
    info!("Display Mode: {:?}", mode);

    Some(DisplayConfig { width, height, pitch: pitch / 4, mode, lfb })
}

pub static DISPLAY: Mutex<Option<DisplayConfig>> = mutex_new!(None);

pub fn display_process(ctx: KernProcessCtx) {
    let mut display = match display_init() {
        None => return,
        Some(d) => d,
    };

    use pi::dma;

    let mut controller = unsafe { dma::Controller::new(1) };

    let mut block = MiniBox::new(&NOCACHE_ALLOC, dma::ControlBlock::new());

    block.source = dma::Source::Constant(display.mode.encode(0xFF0000.into()));
    block.destination = dma::Destination::Increasing(display.lfb.as_mut_ptr(), display.lfb.len());

    let start = pi::timer::current_time();
    controller.execute::<pi::timer::SpinWaiter>(block);

    let diff = pi::timer::current_time() - start;
    info!("Screen DMA took: {:?}", diff);


    let mut p = Painter::new(&display);

    let foo = "Hello world!\nFoo bar baz";

    p.draw_rect(0, 0, 500, 500, 0x4488cc.into());

    p.draw_str(50, 50, foo);

    m_lock!(DISPLAY).replace(display);

    // loop {
    //     // lfb.copy_from_slice(unsafe { core::slice::from_raw_parts(0x80000 as *const u32, lfb.len()) });
    //     kernel_api::syscall::sleep(Duration::from_millis(100));
    // }
}


pub struct GlobalDisplay();

impl GlobalDisplay {
    pub fn new() -> Self {
        Self()
    }
}

impl DisplayHolder for GlobalDisplay {
    fn with_display<T: FnOnce(&mut DisplayConfig)>(&self, func: T) {
        aarch64::dmb();
        if let Some(display) = m_lock!(DISPLAY).as_ref() {
            display.with_display(|d| func(d));
        }
    }
}

