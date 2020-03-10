use alloc::boxed::Box;
use core::ops::Add;
use alloc::string::String;

use kernel_api::{OsError, OsResult};

use aarch64;
use aarch64::SPSR_EL1;
use shim::path::Path;

use crate::console::kprintln;
use crate::FILESYSTEM;
use crate::param::*;
use crate::process::{Stack, State};
use crate::traps::TrapFrame;
use crate::vm::*;
use core::ops::Deref;
use alloc::borrow::ToOwned;
use core::time::Duration;

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
pub struct Process {
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<UserPageTable>,
    /// The scheduling state of the process.
    pub state: State,

    pub name: String,
    
    pub cpu_time: Duration,

    pub kernel_thread_ctx: Option<Box<Box<dyn FnOnce() + Send>>>,
}

type FnTrampoline = extern "C" fn(*mut u8);

// this function expects to be called once. It is the caller's job to
// ensure that the trampoline is only ever called once and that the closure
// outlives the execution of the trampoline.
// unsafe fn kernel_trampoline<F>(closure: &F) -> (FnTrampoline, *mut u8)
//     where
//         F: FnMut(),
// {
//     extern "C" fn trampoline<F>(data: *mut u8)
//         where
//             F: FnMut(),
//     {
//         let closure: &mut F = unsafe { &mut *(data as *mut F) };
//
//         kprintln!("jumping to closure...");
//
//         (*closure)();
//
//         kprintln!("exiting");
//         kernel_api::syscall::exit();
//     }
//
//     (trampoline::<F>, closure as *const F as *mut u8)
// }


unsafe extern "C" fn trampoline(main: *mut u8)
{
    // let closure: &mut F = unsafe { &mut *(data as *mut F) };

    kprintln!("jumping to closure...");

    Box::from_raw(main as *mut Box<dyn FnOnce() + Send>)();

    kprintln!("exiting");
    kernel_api::syscall::exit();
}


impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new(name: String) -> OsResult<Process> {
        let vmap = Box::new(UserPageTable::new());
        let stack = Stack::new().ok_or(OsError::NoMemory)?;
        let context = Box::new(TrapFrame::default());

        Ok(Process {
            context,
            stack,
            vmap,
            state: State::Ready,
            name,
            cpu_time: Duration::from_millis(0),
            kernel_thread_ctx: None,
        })
    }

    pub fn kernel_process(name: String, f: Box<dyn FnOnce() + Send>) -> OsResult<Process> {
        use crate::VMM;

        let ff = Box::new(f);

        let mut p = Process::new(name)?;
        // p.kernel_thread_ctx = Some(ff);

        p.context.sp = p.stack.top().as_u64();

        // let (trampoline, data) = unsafe { kernel_trampoline::<T>(p.kernel_thread_ctx.as_ref().unwrap().as_ref()) };

        let data = ff;

        // C calling conv, first arg in x0.
        p.context.elr = trampoline as u64;
        p.context.regs[0] = &*data as *const Box<dyn FnOnce() + Send> as u64;
        // p.context.regs[1] = size as u64;

        core::mem::forget(data);

        p.context.spsr |= SPSR_EL1::M & 0b0100;

        p.context.ttbr0 = VMM.get_baddr().as_u64();
        // kernel thread still gets a vmap because it's easy
        p.context.ttbr1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    pub fn kernel_process_old(name: String, f: fn()) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::new(name)?;

        p.context.sp = p.stack.top().as_u64();
        p.context.elr = f as u64;

        p.context.spsr |= SPSR_EL1::M & 0b0100;

        p.context.ttbr0 = VMM.get_baddr().as_u64();
        // kernel thread still gets a vmap because it's easy
        p.context.ttbr1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to the its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::do_load(pn)?;

        p.context.sp = Process::get_stack_top().as_u64();
        p.context.elr = USER_IMG_BASE as u64;

        p.context.ttbr0 = VMM.get_baddr().as_u64();
        p.context.ttbr1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use fat32::traits::*;
        use shim::io::Read;
        let mut proc = Process::new(pn.as_ref().to_str().ok_or(OsError::InvalidArgument)?.to_owned())?;

        proc.vmap.alloc(Process::get_stack_base(), PagePerm::RW);

        let mut file = FILESYSTEM.open(pn)?.into_file().ok_or(OsError::InvalidArgument)?;

        let mut base = Process::get_image_base();
        'page_loop: loop {
            let mut buf = proc.vmap.alloc(base, PagePerm::RWX);

            while buf.len() > 0 {
                let read_amount = file.read(buf)?;
                if read_amount == 0 {
                    break 'page_loop;
                }
                buf = &mut buf[read_amount..];
            }

            base = base.add(VirtualAddr::from(PAGE_SIZE));
        }

        Ok(proc)
    }

    /// Returns the highest `VirtualAddr` that is supported by this system.
    pub fn get_max_va() -> VirtualAddr {
        VirtualAddr::from(u64::max_value())
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// memory space.
    pub fn get_image_base() -> VirtualAddr {
        VirtualAddr::from(USER_IMG_BASE as u64)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// process's stack.
    pub fn get_stack_base() -> VirtualAddr {
        VirtualAddr::from(u64::max_value() & PAGE_MASK as u64)
    }

    /// Returns the `VirtualAddr` represents the top of the user process's
    /// stack.
    pub fn get_stack_top() -> VirtualAddr {
        VirtualAddr::from(u64::max_value() & (!0xFu64))
    }

    /// Returns `true` if this process is ready to be scheduled.
    ///
    /// This functions returns `true` only if one of the following holds:
    ///
    ///   * The state is currently `Ready`.
    ///
    ///   * An event being waited for has arrived.
    ///
    ///     If the process is currently waiting, the corresponding event
    ///     function is polled to determine if the event being waiting for has
    ///     occured. If it has, the state is switched to `Ready` and this
    ///     function returns `true`.
    ///
    /// Returns `false` in all other cases.
    pub fn is_ready(&mut self) -> bool {
        if let State::Waiting(h) = &mut self.state {
            let mut copy = core::mem::replace(h, Box::new(|_| false));
            if copy(self) {
                self.state = State::Ready;
            } else {

                // this will always succeed. Cannot re-use h due to lifetimes of passing self
                // into copy()
                if let State::Waiting(h) = &mut self.state {
                    core::mem::replace(h, copy);
                }
            }
        }

        match self.state {
            State::Ready => true,
            _ => false,
        }
    }
}
