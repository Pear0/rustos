use alloc::boxed::Box;
use core::time::Duration;

use crate::console::CONSOLE;
use crate::process::{State, EventPollFn};
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use kernel_api::*;

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {
    let wait_until = pi::timer::current_time() + Duration::from_millis(ms as u64);

    let time_fn: EventPollFn = Box::new(move |p| {
        pi::timer::current_time() >= wait_until
    });
    SCHEDULER.switch(State::Waiting(time_fn), tf);
}

/// Returns current time.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns two
/// parameter:
///  - current time as seconds
///  - fractional part of the current time, in nanoseconds.
pub fn sys_time(tf: &mut TrapFrame) {

    let time = pi::timer::current_time();

    tf.regs[0] = time.as_secs();
    tf.regs[0] = time.subsec_nanos() as u64;

}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    SCHEDULER.kill(tf).expect("killed");
    // we need to schedule a new process otherwise things will be very bad
    SCHEDULER.switch_to(tf);
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {

    CONSOLE.lock().write_byte(b);

}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {

    tf.regs[0] = tf.tpidr;

}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    use crate::console::kprintln;

    match num as usize {
        NR_SLEEP => {
            let time = tf.regs[0];
            sys_sleep(time as u32, tf);
        }
        NR_TIME => {
            sys_time(tf);
        }
        NR_EXIT => {
            sys_exit(tf);
        }
        NR_WRITE => {
            let b = tf.regs[0] as u8;
            sys_write(b, tf)
        }
        NR_GETPID => {
            sys_getpid(tf);
        }
        _ => kprintln!("Unknown syscall: {}", num),
    }
}
