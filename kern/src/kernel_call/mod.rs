
pub mod syscall;

pub const NR_KERN_BASE: usize = 60_000;
pub const NR_WAIT_WAITABLE: usize = NR_KERN_BASE + 0;
pub const NR_YIELD_FOR_TIMERS: usize = NR_KERN_BASE + 1;
pub const NR_EXEC_IN_EXC: usize = NR_KERN_BASE + 2;


