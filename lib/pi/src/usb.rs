extern "C" {
    // fn USPiEnvInitialize() -> i32;

    #[must_use]
    fn USPiInitialize() -> i32;

    #[must_use]
    fn USPiEthernetAvailable() -> i32;

    // fn USPiGetMACAddress(buf: &mut [u8; 6]);

    #[must_use]
    fn USPiEthernetIsLinkUp() -> i32;

    #[must_use]
    fn USPiSendFrame(ptr: *const u8, len: u32) -> i32;

    #[must_use]
    fn USPiReceiveFrame(ptr: *mut u8, len: &mut u32) -> i32;
}


pub const USPI_FRAME_BUFFER_SIZE: usize = 1600;

pub type FrameBuffer = [u8; USPI_FRAME_BUFFER_SIZE];

pub fn new_frame_buffer() -> FrameBuffer {
    [0; USPI_FRAME_BUFFER_SIZE]
}

pub struct Usb();

impl Usb {
    pub unsafe fn new() -> Option<Usb> {
        wrap(USPiInitialize())?;
        Some(Usb())
    }

    pub unsafe fn ethernet_available(&self) -> bool {
        USPiEthernetAvailable() != 0
    }

    pub unsafe fn ethernet_link_up(&self) -> bool {
        USPiEthernetIsLinkUp() != 0
    }

    pub unsafe fn receive_frame<'a>(&self, buf: &'a mut [u8]) -> Option<&'a [u8]> {
        assert!(buf.len() >= USPI_FRAME_BUFFER_SIZE);
        let mut len = 0u32;

        wrap(USPiReceiveFrame(buf.as_mut_ptr(), &mut len))?;

        Some(&buf[..len as usize])
    }

    pub unsafe fn send_frame(&self, buf: &[u8]) -> Option<()> {
        wrap(USPiSendFrame(buf.as_ptr(), buf.len() as u32))
    }
}

fn wrap(num: i32) -> Option<()> {
    match num {
        0 => None,
        _ => Some(()),
    }
}
