use crate::collections::CapacityRingBuffer;
use crate::net::physical::Frame;

pub struct VirtualNIC {

    // guest to hypervisor
    outgoing: CapacityRingBuffer<Frame>,

    // hypervisor to guest
    incoming: CapacityRingBuffer<Frame>,
}

impl VirtualNIC {
    pub fn new() -> Self {
        Self {
            outgoing: CapacityRingBuffer::new(100),
            incoming: CapacityRingBuffer::new(100),
        }
    }
}
