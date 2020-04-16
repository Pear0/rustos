use core::fmt;
use core::fmt::Write;
use core::time::Duration;

use crate::*;

macro_rules! err_or {
    ($ecode:expr, $rtn:expr) => {{
        let e = OsError::from($ecode);
        if let OsError::Ok = e {
            Ok($rtn)
        } else {
            Err(e)
        }
    }};
}

pub fn sleep(span: Duration) -> OsResult<Duration> {
    if span.as_millis() > core::u64::MAX as u128 {
        panic!("too big!");
    }

    let mut ms = span.as_millis() as u64;
    if ms == 0 && span > Duration::default() {
        ms = 1;
    }

    unsafe { do_syscall1r!(NR_SLEEP, ms) }.map(|ms| Duration::from_millis(ms))

}

pub fn sched_yield() {
    sleep(Duration::default());
}

pub fn time() -> Duration {
    let (elapsed_s, elapsed_ns) = unsafe { do_syscall2!(NR_TIME) };
    Duration::new(elapsed_s, elapsed_ns as u32)
}

pub fn exit() -> ! {
    unsafe { do_syscall0!(NR_EXIT); }
    loop{}
}

pub fn write(b: u8) {
    unsafe { do_syscall0!(NR_WRITE, b as u64) };
}

pub fn getpid() -> u64 {
    unsafe { do_syscall1!(NR_GETPID) }
}

pub fn waitpid(pid: u64) -> OsResult<Duration> {
    unsafe { do_syscall1r!(NR_WAITPID, pid) }.map(|ms| Duration::from_millis(ms))
}

pub fn sbrk(increment: i64) -> OsResult<*const u8> {
    unsafe { do_syscall1r!(NR_SBRK, increment as u64) }.map(|addr| addr as *const u8)
}


struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            write(b);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::syscall::vprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
 () => (print!("\n"));
    ($($arg:tt)*) => ({
        $crate::syscall::vprint(format_args!($($arg)*));
        $crate::print!("\n");
    })
}

pub fn vprint(args: fmt::Arguments) {
    let mut c = Console;
    c.write_fmt(args).unwrap();
}
