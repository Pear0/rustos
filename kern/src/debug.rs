
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use xmas_elf::sections::ShType;

use common::fmt::ByteSize;

use crate::{FILESYSTEM2, hw, ALLOCATOR};
use mountfs::mount::mfs;
use shim::path::Path;
use crate::shell::command::CommandError;

#[allow(non_upper_case_globals)]
extern "C" {
    static __code_beg: u8;
    static __code_end: u8;
}

pub fn address_maybe_code(num: u64) -> bool {
    unsafe { num >= (&__code_beg as *const u8 as u64) && num <= (&__code_end as *const u8 as u64) }
}

#[inline(never)]
pub fn stack_scanner(mut sp: usize, stack_top: Option<usize>) -> impl Iterator<Item=u64> {
    sp = crate::allocator::util::align_up(sp, 8);
    let mut top: usize;
    if let Some(t) = stack_top {
        top = t;
    } else {
        top = core::cmp::min(sp + 4096, crate::allocator::util::align_up(sp, 64 * 1024));
    }

    let slice = unsafe { core::slice::from_raw_parts(sp as *const u64, ((top - sp) / 8) as usize) };

    slice.iter().map(|x| *x).filter(|n| address_maybe_code(*n))
}

pub fn read_into_slice_clear<T: Default, I: Iterator<Item=T>>(slice: &mut [T], iter: I) {
    for i in 0..slice.len() {
        slice[i] = T::default();
    }
    read_into_slice(slice, iter)
}

pub fn read_into_slice<T, I: Iterator<Item=T>>(slice: &mut [T], iter: I) {
    for (i, n) in iter.enumerate().take(slice.len()) {
        slice[i] = n;
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct StackFrame {
    pub next: *const StackFrame,
    pub link_register: u64,
}

#[inline(never)]
pub fn base_pointer() -> *const StackFrame {
    let mut bp = 0u64;
    unsafe {
        llvm_asm!("mov $0, x29": "=r"(bp) ::: "volatile");
    }
    bp as *const StackFrame
}

struct StackIter(*const StackFrame);

impl Iterator for StackIter {
    type Item = StackFrame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_null() {
            None
        } else {
            let frame = unsafe { self.0.read() };
            self.0 = frame.next;
            Some(frame)
        }
    }
}

#[inline(never)]
pub fn stack_walker() -> impl Iterator<Item=StackFrame> {
    let ptr = base_pointer();
    StackIter(ptr)
}

/// Simple unsafe container to hold globally allocated debug data.
struct UnsafeContainer<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for UnsafeContainer<T> {}

impl<T> UnsafeContainer<T> {
    pub const fn new(value: T) -> Self {
        Self { inner: UnsafeCell::new(value) }
    }
}

pub struct MetaDebugInfo {
    file_buffer: &'static [u8],
    pub context: addr2line::Context<gimli::EndianSlice<'static, gimli::RunTimeEndian>>,
}

static SELF_SYMBOLS: UnsafeContainer<Option<Box<MetaDebugInfo>>> = UnsafeContainer::new(None);

fn load_elf_pi() -> Result<&'static mut Vec<u8>, crate::shell::command::CommandError> {

    let mut entry: Box<dyn mfs::File> = FILESYSTEM2.open("/kernel.elf")?.into_file().ok_or("no file")?;
    info!("opened file: {}", entry.name());

    let file_buffer = Box::leak(Box::new(Vec::<u8>::new()));

    use shim::io;
    io::copy(entry.as_mut(), file_buffer);

    Ok(file_buffer)
}

fn load_elf_khadas() -> Result<&'static mut Vec<u8>, crate::shell::command::CommandError> {
    use compression::prelude::*;
    let compressed_elf_location = 0x4000000u64;
    let max_size = 16 * 1024 * 1024;

    let compressed_slice = unsafe { core::slice::from_raw_parts(compressed_elf_location as *const u8, max_size) };

    // make a copy, carefully under the 64mb limit. Now the compressed copy from u-boot can be overridden.
    let mut compressed_vec: Vec<u8> = Vec::new();
    compressed_vec.reserve_exact(max_size);
    compressed_vec.resize(max_size, 0);
    compressed_vec.copy_from_slice(compressed_slice);

    let alloc_wilderness = ALLOCATOR.with_internal(|a| a.wilderness());
    if (alloc_wilderness.0 as u64) > compressed_elf_location {
        warn!("Allocator wilderness {:#x} > elf location {:#x}, corruption likely", alloc_wilderness.0, compressed_elf_location);
    } else {
        info!("Allocator wilderness {:#x} > elf location {:#x}, probably no corruption", alloc_wilderness.0, compressed_elf_location);
    }

    let comp: Vec<_> = compressed_vec.iter().cloned().decode(&mut GZipDecoder::new()).collect::<Result<Vec<_>, _>>()?;

    info!("Total debug info len: {}", comp.len());

    let file_buffer = Box::leak(Box::new(comp));

    Ok(file_buffer)
}

fn load_elf_symbols(file_buffer: &'static mut Vec<u8>) -> Result<(), crate::shell::command::CommandError> {
    let elf = xmas_elf::ElfFile::new(file_buffer.as_slice())?;

    // for section in elf.section_iter() {
    //     let typ = section.get_type()?;
    //     let name = if let ShType::Null = typ { "<null>" } else { section.get_name(&elf)? };
    //
    //     info!("  S: {} - {:?}", name, typ);
    // }

    let loader = |section: gimli::SectionId| {
        match elf.find_section_by_name(section.name()) {
            Some(s) => {
                Ok(s.raw_data(&elf))
            }
            None => {
                if false {
                    use alloc::format;
                    let s = Box::new(format!("Not found section: {}", section.name()));
                    let s = Box::leak(s);
                    return Err(s.as_str());
                }

                Ok(&[][..])
            }
        }
    };

    let sup_loader = |_: gimli::SectionId| Ok(&[][..]);

    info!("gimli::load()");

    let dwarf_slice = gimli::Dwarf::load(loader, sup_loader)?;

    let borrow_section: &dyn for<'a> Fn(&&'a [u8]) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, gimli::RunTimeEndian::default());

    // Create `EndianSlice`s for all of the sections.
    let dwarf = dwarf_slice.borrow(&borrow_section);

    info!("addr2line::Context::from_dwarf()");

    let context = addr2line::Context::from_dwarf(dwarf)?;

    info!("registering global symbols");

    unsafe { SELF_SYMBOLS.inner.get().write(Some(Box::new(MetaDebugInfo { file_buffer, context }))) };

    // Ensure other threads see our change.
    aarch64::clean_data_cache_obj(&SELF_SYMBOLS);

    Ok(())
}


fn do_initialize() -> Result<(), crate::shell::command::CommandError> {

    let file_buffer = if hw::not_pi() {
        match hw::arch_variant() {
            hw::ArchVariant::Khadas(_) => load_elf_khadas()?,
            _ => Err("cannot load debug symbols for unknown arch")?,
        }
    } else {
        load_elf_pi()?
    };

    info!("Read {} bytes", ByteSize::from(file_buffer.len()));

    load_elf_symbols(file_buffer)
}

pub fn debug_ref() -> Option<&'static MetaDebugInfo> {
    unsafe {
        SELF_SYMBOLS.inner.get()
            .as_ref().unwrap()
            .as_ref()
            .map(|x| x.as_ref())
    }
}

pub fn initialize_debug() {

    if debug_ref().is_some() {
        warn!("Refusing to initialize debug information, already init.");
        return;
    }

    match do_initialize() {
        Ok(()) => {},
        Err(e) => {
            error!("Failed to load debug info: {:?}", e);
        }
    }
}

pub fn load_from_file(path: &dyn AsRef<Path>) -> Result<(), CommandError> {
    if debug_ref().is_some() {
        warn!("Refusing to initialize debug information, already init.");
        return Ok(())
    }

    let mut entry: Box<dyn mfs::File> = FILESYSTEM2.open(path)?.into_file().ok_or("no file")?;
    info!("opened file: {}", entry.name());

    let file_buffer = Box::leak(Box::new(Vec::<u8>::new()));

    use shim::io;
    io::copy(entry.as_mut(), file_buffer)?;

    info!("Read {} bytes", ByteSize::from(file_buffer.len()));

    load_elf_symbols(file_buffer)?;

    Ok(())
}























