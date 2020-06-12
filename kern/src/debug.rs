
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use xmas_elf::sections::ShType;

use common::fmt::ByteSize;

use crate::FILESYSTEM;

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

fn do_initialize() -> Result<(), crate::shell::command::CommandError> {
    use fat32::traits::{Entry, FileSystem};
    type File = fat32::vfat::File<crate::fs::PiVFatHandle>;

    let mut entry: File = FILESYSTEM.open("/kernel.elf")?.into_file().ok_or("no file")?;
    info!("opened file: {}", entry.name);

    let file_buffer = Box::leak(Box::new(Vec::<u8>::new()));

    use shim::io;
    io::copy(&mut entry, file_buffer);

    info!("Read {} bytes", ByteSize::from(file_buffer.len()));

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

    let dwarf_slice = gimli::Dwarf::load(loader, sup_loader)?;

    let borrow_section: &dyn for<'a> Fn(&&'a [u8]) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, gimli::RunTimeEndian::default());

    // Create `EndianSlice`s for all of the sections.
    let dwarf = dwarf_slice.borrow(&borrow_section);

    let context = addr2line::Context::from_dwarf(dwarf)?;

    unsafe { SELF_SYMBOLS.inner.get().write(Some(Box::new(MetaDebugInfo { file_buffer, context }))) };

    // Ensure other threads see our change.
    aarch64::clean_data_cache_obj(&SELF_SYMBOLS);

    Ok(())
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























