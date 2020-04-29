use pi::usb;

#[derive(Debug, PartialEq, Eq)]
pub enum LinkStatus {
    /// Network is down for an unknown reason
    UnknownDown,

    /// Network is disconnected, ie. a physical cable is not connected.
    Disconnected,

    /// Network is connected but a physical link has not been established.
    LinkDown,

    /// Link is up.
    Up,
}

pub const FRAME_BUFFER_SIZE: usize = 1600;
pub type RawBuffer = [u8; FRAME_BUFFER_SIZE];

pub struct Frame(RawBuffer, usize);

pub trait Physical {

    fn status(&self) -> LinkStatus;

    fn send_frame(&self, frame: &Frame) -> Option<()>;

    fn receive_frame(&self, frame: &mut Frame) -> Option<()>;

    // Provided functions

    fn is_connected(&self) -> bool {
        self.status() == LinkStatus::Up
    }
}

pub struct PhysicalUsb(pub usb::Usb);

impl Physical for PhysicalUsb {
    fn status(&self) -> LinkStatus {
        unsafe {
            if !self.0.ethernet_available() {
                return LinkStatus::Disconnected;
            }

            if !self.0.ethernet_link_up() {
                return LinkStatus::LinkDown;
            }

            LinkStatus::Up
        }
    }

    fn send_frame(&self, frame: &Frame) -> Option<()> {
        unsafe { self.0.send_frame( &frame.0[..frame.1]) }
    }

    fn receive_frame(&self, frame: &mut Frame) -> Option<()> {
        frame.1 = 0;
        let buf = unsafe { self.0.receive_frame(&mut frame.0) }?;
        frame.1 = buf.len();
        Some(())
    }
}



