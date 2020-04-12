use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;
use core::fmt;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ProcessBundle {
    pub frame: Vec<u8>,
    pub name: String,
    pub memory: MemoryBundle,
}

impl fmt::Debug for ProcessBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessBundle")
            .field("frame", &"<omitted>")
            .field("name", &self.name)
            .field("memory", &"<omitted>")
            .finish()
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MemoryBundle {
    pub generic_pages: HashMap<u64, Vec<u8>>,
}





