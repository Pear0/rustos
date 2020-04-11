use alloc::string::String;
use alloc::vec::Vec;

use crate::traps::TrapFrame;
use hashbrown::HashMap;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ProcessBundle {
    pub frame: Vec<u8>,
    pub name: String,
    pub memory: MemoryBundle,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MemoryBundle {
    pub generic_pages: HashMap<u64, Vec<u8>>,
}





