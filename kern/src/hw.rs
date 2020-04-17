use crate::mbox::with_mbox;
use pi::mbox::MBox;
use pi::atags::{Atags, Atag};
use aarch64::attr;

pub fn is_qemu() -> bool {
    let mut aes = false;
    let mut sha1 = false;
    let mut sha2 = false;

    // heuristically detect QEMU based on supported crypto extensions.
    // The Raspberry Pi only supports CRC32 but QEMU reports support for
    // additional crypto extensions.

    for attr in attr::iter_enabled() {
        use attr::Attribute::*;
        match attr {
            AES => aes = true,
            SHA1 => sha1 = true,
            SHA2 => sha2 = true,
            _ => {},
        }
    }

    aes && sha1 && sha2
}


