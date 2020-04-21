use alloc::boxed::Box;
use core::time::Duration;

use crate::console::CONSOLE;
use crate::process::{State, EventPollFn};
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use kernel_api::*;

fn set_result(tf: &mut TrapFrame, regs: &[u64]) {
    for (i, v) in regs.iter().enumerate() {
        tf.regs[i] = *v;
    }
}

fn set_err(tf: &mut TrapFrame, res: OsError) {
    tf.regs[7] = res as u64;
}

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {
    if ms == 0 {
        SCHEDULER.switch(State::Ready, tf);
    } else {
        let start = pi::timer::current_time();
        let wait_until = start + Duration::from_millis(ms as u64);

        let time_fn: EventPollFn = Box::new(move |tf| {
            let now = pi::timer::current_time();
            let good = now >= wait_until;
            if good {
                let d = (now - start).as_millis() as u64;
                set_result(&mut tf.context, &[d]);
                set_err(&mut tf.context, OsError::Ok);
            }
            good
        });
        SCHEDULER.switch(State::Waiting(time_fn), tf);
    }
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

    if b == b'\n' {
        CONSOLE.lock().write_byte(b'\r');
    }
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
