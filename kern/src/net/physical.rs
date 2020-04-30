use pi::usb;
use crate::net::{ether, try_parse_struct};
use core::fmt;
use kernel_api::hypercall::*;
use kernel_api::OsError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone)]
pub struct Frame(pub RawBuffer, pub usize);

impl Default for Frame {
    fn default() -> Self {
        Self([0; FRAME_BUFFER_SIZE], 0)
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Frame").field(&self.as_slice()).finish()
    }
}


impl Frame {
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..self.1]
    }

    pub fn eth(&self) -> Option<ether::EthHeader> {
        try_parse_struct::<ether::EthHeader>(self.as_slice())
            .map(|(header, _)| header)
    }
}

pub trait Physical : Sync + Send {

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
        if buf.len() >= 14 {
            debug!("receive frame {:x} {:x}", buf[12], buf[13]);
        }
        frame.1 = buf.len();
        Some(())
    }
}

pub struct VirtNIC();

impl Physical for VirtNIC {
    fn status(&self) -> LinkStatus {
        match vnic_state() {
            Ok(true) => LinkStatus::Up,
            Ok(false) => LinkStatus::LinkDown,
            Err(e) => {
                error!("VirtNIC::status(): {:?}", e);
                LinkStatus::UnknownDown
            }
        }
    }

    fn send_frame(&self, frame: &Frame) -> Option<()> {
        match vnic_send_frame(frame.as_slice()) {
            Ok(()) => Some(()),
            Err(e) => {
                error!("VirtNIC::send_frame(): {:?}", e);
                None
            }
        }
    }

    fn receive_frame(&self, frame: &mut Frame) -> Option<()> {
        frame.1 = 0;
        match vnic_receive_frame(&mut frame.0) {
            Ok(len) => {
                frame.1 = len;
                Some(())
            },
            Err(OsError::Waiting) => None,
            Err(e) => {
                error!("VirtNIC::receive_frame(): {:?}", e);
                None
            }
        }
    }
}

