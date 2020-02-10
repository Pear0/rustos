use crate::atags::raw;

pub use crate::atags::raw::{Core, Mem};
use core::intrinsics::size_of;

/// An ATAG.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Atag {
    Core(raw::Core),
    Mem(raw::Mem),
    Cmd(&'static str),
    Unknown(u32),
    None,
}

impl Atag {
    /// Returns `Some` if this is a `Core` ATAG. Otherwise returns `None`.
    pub fn core(self) -> Option<Core> {
        match self {
            Atag::Core(s) => Some(s),
            _ => None,
        }
    }

    /// Returns `Some` if this is a `Mem` ATAG. Otherwise returns `None`.
    pub fn mem(self) -> Option<Mem> {
        match self {
            Atag::Mem(s) => Some(s),
            _ => None,
        }
    }

    /// Returns `Some` with the command line string if this is a `Cmd` ATAG.
    /// Otherwise returns `None`.
    pub fn cmd(self) -> Option<&'static str> {
        match self {
            Atag::Cmd(s) => Some(s),
            _ => None,
        }
    }
}

fn parse_cmd(cmd: &raw::Cmd) -> &'static str {
    let mut size = 0usize;
    let ptr = &cmd.cmd as *const u8;

    while unsafe { *(ptr.add(size)) } != b'\0' {
        size += 1;
    }

    let buf = unsafe { core::slice::from_raw_parts(&cmd.cmd, size) };
    unsafe { core::str::from_utf8_unchecked(buf) }
}

impl From<&'static raw::Atag> for Atag {
    fn from(atag: &'static raw::Atag) -> Atag {

        unsafe {
            match (atag.tag, &atag.kind) {
                (raw::Atag::CORE, &raw::Kind { core }) => Atag::Core(core),
                (raw::Atag::MEM, &raw::Kind { mem }) =>  Atag::Mem(mem),
                (raw::Atag::CMDLINE, &raw::Kind { ref cmd }) =>  Atag::Cmd(parse_cmd(cmd)),
                (raw::Atag::NONE, _) => Atag::None,
                (id, _) => Atag::Unknown(id),
            }
        }
    }
}
