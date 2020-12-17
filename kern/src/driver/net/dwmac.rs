use core::fmt;
use core::time::Duration;

use dsx::sync::mutex::LockableMutex;
use dwmac::Gmac;

use crate::mini_allocators::NOCACHE_PAGE_ALLOC;
use crate::mutex::Mutex;
use crate::net::physical::{Frame, LinkStatus, Physical};
use crate::timing;
use crate::virtualization::AccessSize::DoubleWord;

#[derive(Default)]
struct DwMacHooks;

impl dwmac::Hooks for DwMacHooks {
    fn sleep(dur: Duration) {
        timing::sleep_phys(dur);
    }

    fn system_time() -> Duration {
        timing::clock_time_phys()
    }

    fn memory_barrier() {
        aarch64::dsb();
    }

    fn flush_cache(addr: u64, len: u64, flush: dwmac::FlushType) {
        match flush {
            dwmac::FlushType::Clean => aarch64::clean_data_cache_region(addr, len),
            dwmac::FlushType::Invalidate => aarch64::invalidate_data_cache_region(addr, len),
            dwmac::FlushType::CleanAndInvalidate => aarch64::clean_and_invalidate_data_cache_region(addr, len),
        }
    }
}

pub struct DwMac1000 {
    gmac: Mutex<Gmac<DwMacHooks>>,
}

impl DwMac1000 {
    pub fn open() -> Result<DwMac1000, dwmac::Error> {
        Ok(DwMac1000 {
            gmac: Mutex::new(Gmac::<DwMacHooks>::open(&NOCACHE_PAGE_ALLOC)?)
        })
    }
}

impl Physical for DwMac1000 {
    fn status(&self) -> LinkStatus {
        LinkStatus::Up
    }

    fn send_frame(&self, frame: &Frame) -> Option<()> {
        let mut gmac = self.gmac.lock();
        debug!("sending frame... {}", frame.1);
        if let Err(e) = gmac.transmit_frame(frame.as_slice()) {
            info!("failed to send frame: {:?}", e);
            return None;
        }
        Some(())
    }

    fn receive_frame(&self, frame: &mut Frame) -> Option<()> {
        let mut result: Option<()> = None;
        let mut gmac = self.gmac.lock();
        gmac.receive_frames(1, &mut |slice| {
            (&mut frame.0[..slice.len()]).copy_from_slice(slice);
            frame.1 = slice.len();
            result = Some(());
            info!("read packet len: {}", slice.len());
        }).ok()?;
        result
    }

    fn name(&self) -> &'static str {
        "DwMac1000"
    }

    fn debug_dump(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let mut gmac = self.gmac.lock();
        gmac.debug_dump(w)
    }
}


