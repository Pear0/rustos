use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU8, Ordering};
use core::time::Duration;

use dsx::sync::mutex::LockableMutex;
use spin::RwLock;
use usb_host::{USBErrorKind, USBHost, USBResult};
use usb_host::consts::USBSpeed;
use usb_host::drivers::hub::HubDriver;
use usb_host::drivers::keyboard::{HIDKeyboard, HIDKeyboardCallback};
use usb_host::drivers::mass_storage::{MassStorageDriver, MSDCallback, SimpleBlockDevice, TransparentSCSI};
use usb_host::structs::{DeviceState, USBDevice};
use xhci::FlushType;

use fat32::traits::BlockDevice;
use fat32::vfat::{DynVFatHandle, DynWrapper, VFat};
use shim::io;
use shim::path::PathBuf;

use crate::{can_make_syscall, FILESYSTEM2, timing};
use crate::arm::PhysicalCounter;
use crate::hw::{self, ArchVariant};
use crate::iosync::Global;
use crate::process::KernProcessCtx;

type BoxFn = Box<dyn FnOnce() + Send>;

static EVENT_QUEUE: Global<VecDeque<BoxFn>> = Global::new(|| VecDeque::new());

struct XHCIHal();

impl usb_host::UsbHAL for XHCIHal {
    fn sleep(dur: Duration) {
        let syscall_threshold = Duration::from_millis(1);

        if can_make_syscall() && dur >= syscall_threshold {
            kernel_api::syscall::sleep(dur);
        } else {
            timing::sleep_phys(dur);
        }
    }

    fn current_time() -> Duration {
        timing::clock_time::<PhysicalCounter>()
    }

    fn queue_task(func: Box<dyn FnOnce() + Send>) {
        EVENT_QUEUE.critical(|e| e.push_back(func));
    }
}

impl xhci::XhciHAL for XHCIHal {
    fn memory_barrier() {
        aarch64::dmb();
    }

    fn translate_addr(addr: u64) -> u64 {
        addr
    }

    fn flush_cache(addr: u64, len: u64, flush: FlushType) {
        match flush {
            FlushType::Clean => aarch64::clean_data_cache_region(addr, len),
            FlushType::Invalidate => aarch64::invalidate_data_cache_region(addr, len),
            FlushType::CleanAndInvalidate => aarch64::clean_and_invalidate_data_cache_region(addr, len),
        }
    }
}

struct HIDCallbacks;

impl HIDKeyboardCallback for HIDCallbacks {
    fn key_down(ascii: u8) {
        info!("key press: {}", String::from_utf8_lossy(core::slice::from_ref(&ascii)));
    }
}

struct USBDriver();

impl usb_host::HostCallbacks<XHCIHal> for USBDriver {
    fn new_device(&self, host: &Arc<USBHost<XHCIHal>>, device: &Arc<RwLock<USBDevice>>) -> USBResult<()> {
        use usb_host::consts::*;
        type X = USBHost<XHCIHal>;

        let (device_desc, configuration) = {
            let d = device.read();
            (d.ddesc.clone(), d.config_desc.as_ref().ok_or(USBErrorKind::InvalidArgument.msg("expected config descriptor"))?.clone())
        };

        let mfg = X::fetch_string_descriptor(device, device_desc.iManufacturer, 0x409).unwrap_or(String::from("(no manufacturer name)"));
        let prd = X::fetch_string_descriptor(device, device_desc.iProduct, 0x409).unwrap_or(String::from("(no product name)"));
        let serial = X::fetch_string_descriptor(device, device_desc.iSerialNumber, 0x409).unwrap_or(String::from("(no serial number)"));
        debug!("[XHCI] New device:\n  MFG: {}\n  Prd:{}\n  Serial:{}", mfg, prd, serial);

        for interface in &configuration.ifsets {
            if interface.interface.bAlternateSetting != 0 {
                debug!("Skipping non-default altSetting Interface");
                continue;
            }

            if let Err(e) = HubDriver::<XHCIHal>::probe(host, &device, interface) {
                error!("failed to probe hub: {:?}", e);
            }
            {
                let d = device.read();
                if matches!(d.device_state, DeviceState::Owned(_)) {
                    break;
                }
            }
            if let Err(e) = MassStorageDriver::<XHCIHal, MassFSHook>::probe(&device, interface) {
                error!("failed to probe msd: {:?}", e);
            }
            {
                let d = device.read();
                if matches!(d.device_state, DeviceState::Owned(_)) {
                    break;
                }
            }
            if let Err(e) = HIDKeyboard::<XHCIHal, HIDCallbacks>::probe(&device, interface) {
                error!("failed to probe msd: {:?}", e);
            }
            {
                let d = device.read();
                if matches!(d.device_state, DeviceState::Owned(_)) {
                    break;
                }
            }
            // match interface.interface.bInterfaceClass {
            //     CLASS_CODE_HID => {
            //         if let Err(e) = HIDKeyboard::<XHCIHal>::probe(&device, interface) {
            //             error!("failed to probe hid: {:?}", e);
            //         }
            //     }
            //     _ => {}
            // }
        }

        Ok(())
    }
}

struct SCSIWrapper(TransparentSCSI);

impl BlockDevice for SCSIWrapper {
    fn sector_size(&self) -> u64 {
        self.0.sector_size()
    }

    fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read_sector(n, buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e.msg))
    }

    fn write_sector(&mut self, n: u64, buf: &[u8]) -> io::Result<usize> {
        self.0.write_sector(n, buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e.msg))
    }
}


pub struct MassFSHook();

static FOO: AtomicU8 = AtomicU8::new(b'a');

impl MSDCallback for MassFSHook {
    fn on_new_scsi(scsi: TransparentSCSI) -> USBResult<()> {
        info!("called hook on: {:?}", scsi.get_capacity());

        let bd = SCSIWrapper(scsi);

        let vfat = match VFat::<DynVFatHandle>::from(bd) {
            Ok(v) => v,
            Err(e) => {
                debug!("failed to init vfat: {:?}", e);
                return Ok(());
            }
        };

        let mut string = String::from("/drive");
        string.push(char::from(FOO.fetch_add(1, Ordering::Relaxed)));

        let mut f_lock = FILESYSTEM2.0.lock();
        let f = f_lock.as_mut().expect("FS2 not initialized");
        f.mount(Some(&PathBuf::from(string)), Box::new(DynWrapper(vfat)));


        Ok(())
    }
}

fn process_event_queue() {
    // bound the maximum work we will perform before yielding
    let len = EVENT_QUEUE.critical(|e| e.len());
    for _ in 0..len {
        if let Some(func) = EVENT_QUEUE.critical(|e| e.pop_front()) {
            func();
        } else {
            return;
        }
    }
}

pub fn usb_thread(ctx: KernProcessCtx) {
    use usb_host::traits::*;

    if !matches!(hw::arch_variant(), ArchVariant::Khadas(_)) {
        return;
    }

    let addr = 0xff500000u64;

    xhci::init_dwc3(addr);

    let xx = xhci::Xhci::<XHCIHal>::new(addr);

    let my_xhci = Arc::new(xhci::XhciWrapper::<XHCIHal>(spin::Mutex::new(xx)));

    info!("created things");

    let host = USBHost::<XHCIHal>::new(Arc::new(USBDriver()));
    let dev = host.attach_root_hub(my_xhci.clone(), USBSpeed::Super);

    let host = Arc::new(host);

    USBHost::<XHCIHal>::setup_new_device(&host, dev);


    loop {
        my_xhci.process_interrupts();
        process_event_queue();
        kernel_api::syscall::sleep(Duration::from_millis(5));
    }

    // smp::no_interrupt(|| {
    //     match xx.do_stuff() {
    //         Ok(()) => info!("did stuff successfully"),
    //         Err(e) => error!("Error failed to do stuff: {:?}", e),
    //     }
    // });
}




