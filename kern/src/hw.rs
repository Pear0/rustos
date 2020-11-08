use crate::mbox::with_mbox;
use pi::mbox::MBox;
use pi::atags::{Atags, Atag};
use aarch64::attr;
use karch::Arch;

#[derive(Default, Clone)]
pub struct ArchInitInfo {
    pub entry_regs: [u64; 3],
}

fn has_crypto() -> bool {
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

pub fn is_qemu() -> bool {
    !matches!(arch_variant(), &ArchVariant::Khadas(_)) && has_crypto()
}

pub fn not_pi() -> bool {
    true
}

static CORE_DMA_CONTROLLERS: usize = 0;

pub enum ArchVariant {
    Uninit,
    Khadas(khadas::KhadasArch),
    Pi(pi::PiArch),
}

static mut BIGLY_ARCH: ArchVariant = ArchVariant::Uninit;

pub unsafe fn init_hal(info: ArchInitInfo) {
    if let Some(arch) = pi::PiArch::new() {
        BIGLY_ARCH = ArchVariant::Pi(arch);
        return;
    }

    if let Some(arch) = khadas::KhadasArch::new(info.entry_regs[0]) {
        BIGLY_ARCH = ArchVariant::Khadas(arch);
        return;
    }

    aarch64::halt_loop();
}

pub fn maybe_arch() -> Option<&'static dyn Arch> {
    unsafe {
        match &BIGLY_ARCH {
            ArchVariant::Khadas(khadas) => Some(khadas),
            ArchVariant::Pi(arch) => Some(arch),
            ArchVariant::Uninit => None,
        }
    }
}

pub fn arch() -> &'static dyn Arch {
    if let Some(arch) = maybe_arch() {
        arch
    } else {
        aarch64::halt_loop();
    }
}

pub fn arch_variant() -> &'static ArchVariant {
    unsafe { &BIGLY_ARCH }
}







