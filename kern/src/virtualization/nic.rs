use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::time::Duration;

use hashbrown::HashMap;

use pi::timer;

use crate::collections::CapacityRingBuffer;
use crate::mutex::Mutex;
use crate::net::ether;
use crate::net::physical::{Frame, LinkStatus, Physical};

struct VirtualNICImpl {
    status: LinkStatus,

    // guest to hypervisor
    outgoing: CapacityRingBuffer<Frame>,

    // hypervisor to guest
    incoming: CapacityRingBuffer<Frame>,
}

pub struct VirtualNIC(Mutex<VirtualNICImpl>);

impl VirtualNIC {
    pub fn new() -> Self {
        Self(mutex_new!(VirtualNICImpl {
            status: LinkStatus::Disconnected,
            outgoing: CapacityRingBuffer::new(100),
            incoming: CapacityRingBuffer::new(100),
        }))
    }

    pub fn set_status(&self, status: LinkStatus) {
        m_lock!(self.0).status = status;
    }
}

impl Physical for VirtualNIC {
    fn status(&self) -> LinkStatus {
        m_lock!(self.0).status
    }

    fn send_frame(&self, frame: &Frame) -> Option<()> {
        if m_lock!(self.0).outgoing.insert(frame.clone()) {
            Some(())
        } else {
            None
        }
    }

    fn receive_frame(&self, frame: &mut Frame) -> Option<()> {
        if let Some(f) = m_lock!(self.0).incoming.remove() {
            *frame = f;
            Some(())
        } else {
            None
        }
    }
}


#[repr(C)]
pub struct RevVirtualNIC(pub VirtualNIC);

impl RevVirtualNIC {
    pub fn create(arc: Arc<VirtualNIC>) -> Arc<RevVirtualNIC> {
        unsafe { core::mem::transmute(arc) }
    }
}

impl Physical for RevVirtualNIC {
    fn status(&self) -> LinkStatus {
        m_lock!((self.0).0).status
    }

    fn send_frame(&self, frame: &Frame) -> Option<()> {
        if m_lock!((self.0).0).incoming.insert(frame.clone()) {
            Some(())
        } else {
            None
        }
    }

    fn receive_frame(&self, frame: &mut Frame) -> Option<()> {
        if let Some(f) = m_lock!((self.0).0).outgoing.remove() {
            *frame = f;
            Some(())
        } else {
            None
        }
    }
}

type NIC = Weak<dyn Physical>;

pub struct VirtualSwitch {
    nics: HashMap<usize, NIC>,
    mac_memory: HashMap<ether::Mac, (usize, NIC)>,
    id_counter: usize,
    hub_mode: bool,
    pub debug: bool,
}

impl VirtualSwitch {
    pub fn new() -> Self {
        Self {
            nics: HashMap::new(),
            mac_memory: HashMap::new(),
            id_counter: 1,
            hub_mode: false,
            debug: false,
        }
    }

    pub fn new_hub() -> Self {
        Self {
            nics: HashMap::new(),
            mac_memory: HashMap::new(),
            id_counter: 1,
            hub_mode: true,
            debug: false,
        }
    }

    pub fn register(&mut self, nic: Arc<dyn Physical>) -> usize {
        let id = self.id_counter;
        self.id_counter += 1;
        self.nics.insert(id, Arc::downgrade(&nic));
        id
    }

    fn vacuum(&mut self) {
        let mut needs_vacuuming = false;

        // are greater than 50% of the NICs dead Arcs
        {
            let mut count = 0;
            for v in self.nics.values() {
                if let None = v.upgrade() {
                    count += 1;
                }
            }
            if count > (self.nics.len() + 1) / 2 {
                needs_vacuuming = true;
            }
        }

        // are greater than 50% of the MAC address lookup entries dead
        {
            let mut count = 0;
            for (_, v) in self.mac_memory.values() {
                if let None = v.upgrade() {
                    count += 1;
                }
            }
            if count > (self.mac_memory.len() + 1) / 2 {
                needs_vacuuming = true;
            }
        }

        // rebuild hash maps.
        if needs_vacuuming {
            self.nics = core::mem::replace(&mut self.nics, HashMap::new()).into_iter().filter(|(_, nic)| nic.upgrade().is_some()).collect();
            self.mac_memory = core::mem::replace(&mut self.mac_memory, HashMap::new()).into_iter().filter(|(_, (_, nic))| nic.upgrade().is_some()).collect();
        }
    }

    fn send_frame(&self, frame: &Frame, exclude: Option<usize>) {
        let header = match frame.eth() {
            Some(h) => h,
            None => return,
        };

        if self.debug {
            debug!("[{:x}] {} -> {}", header.protocol_type.get(), header.mac_sender, header.mac_receiver);
        }

        if !self.hub_mode {
            // try to send only to one NIC
            if !header.mac_receiver.is_broadcast() {
                if let Some((_, receiver)) = self.mac_memory.get(&header.mac_receiver) {
                    if let Some(receiver) = receiver.upgrade() {
                        receiver.send_frame(frame);
                        return;
                    }
                }
            }
        }

        // otherwise send to all NICs, except the excluded the NIC
        for (id, receiver) in self.nics.iter() {
            if Some(*id) != exclude {
                if let Some(receiver) = receiver.upgrade() {
                    receiver.send_frame(frame);
                    return;
                }
            }
        }
    }

    pub fn process(&mut self, time_budget: Duration) -> bool {
        let end = timer::current_time() + time_budget;

        self.vacuum();

        let mut first = true;
        while first || timer::current_time() < end {
            first = false;
            let mut processed = false;
            for (send_id, sender) in self.nics.iter() {
                if let Some(sender) = sender.upgrade() {
                    let mut frame = Frame::default();
                    if sender.receive_frame(&mut frame).is_some() {
                        processed = true;

                        if !self.hub_mode {
                            // remember who sends from a mac address
                            if let Some(eth) = frame.eth() {
                                self.mac_memory.insert(eth.mac_sender, (*send_id, Arc::downgrade(&sender)));
                            }
                        }

                        self.send_frame(&frame, Some(*send_id));
                    }
                }
            }
            if !processed {
                return false; // no more work to do
            }
        }

        true
    }
}

