use alloc::boxed::Box;
use core::time::Duration;
use alloc::sync::Arc;

use kernel_api::*;
use crate::kernel_call::*;

use crate::console::CONSOLE;
use crate::kernel::KERNEL_SCHEDULER;
use crate::process::{EventPollFn, State, KernelImpl};
use crate::{process};
use crate::traps::KernelTrapFrame;
use crate::sync::{Completion, Waitable};
use crate::param::PAGE_SIZE;


fn set_result(tf: &mut KernelTrapFrame, regs: &[u64]) {
    for (i, v) in regs.iter().enumerate() {
        tf.regs[i] = *v;
    }
}

fn set_err(tf: &mut KernelTrapFrame, res: OsError) {
    tf.regs[7] = res as u64;
}

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut KernelTrapFrame) {
    if ms == 0 {
        KERNEL_SCHEDULER.switch(State::Ready, tf);
    } else {
        let start = pi::timer::current_time();
        let wait_until = start + Duration::from_millis(ms as u64);

        let time_fn: EventPollFn<KernelImpl> = Box::new(move |tf| {
            let now = pi::timer::current_time();
            let good = now >= wait_until;
            if good {
                let d = (now - start).as_millis() as u64;
                set_result(&mut tf.context, &[d]);
                set_err(&mut tf.context, OsError::Ok);
            }
            good
        });
        KERNEL_SCHEDULER.switch(State::Waiting(time_fn), tf);
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
pub fn sys_time(tf: &mut KernelTrapFrame) {

    let time = pi::timer::current_time();

    tf.regs[0] = time.as_secs();
    tf.regs[0] = time.subsec_nanos() as u64;

}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut KernelTrapFrame) {
    KERNEL_SCHEDULER.kill(tf).expect("killed");
    // we need to schedule a new process otherwise things will be very bad
    KERNEL_SCHEDULER.switch_to(tf);
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, _tf: &mut KernelTrapFrame) {

    if b == b'\n' {
        m_lock!(CONSOLE).write_byte(b'\r');
    }
    m_lock!(CONSOLE).write_byte(b);

}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut KernelTrapFrame) {

    tf.regs[0] = tf.tpidr;

}

pub fn sys_waitpid(pid: u64, tf: &mut KernelTrapFrame) {
    let start = pi::timer::current_time();

    let comp = Arc::new(Completion::<process::Id>::new());

    let comp_clone = comp.clone();
    let did_register = KERNEL_SCHEDULER.crit_process(pid, move |proc| {
        if let Some(proc) = proc {
            proc.detail.dead_completions.push(comp_clone);
            true
        } else {
            comp_clone.complete(pid);
            false
        }
    });

    let time_fn: EventPollFn<KernelImpl> = Box::new(move |tf| {
        let now = pi::timer::current_time();
        if comp.get().is_some() {
            let d = (now - start).as_millis() as u64;
            set_result(&mut tf.context, &[d]);
            set_err(&mut tf.context, if did_register { OsError::Ok } else { OsError::InvalidArgument });
            true
        } else {
            false
        }
    });
    KERNEL_SCHEDULER.switch(State::Waiting(time_fn), tf);
}

pub fn sys_wait_waitable(tf: &mut KernelTrapFrame) {
    // TODO insecure (can be called from userspace)
    let arc: Arc<dyn Waitable> = unsafe { core::mem::transmute([tf.regs[0], tf.regs[1]]) };
    KERNEL_SCHEDULER.switch(State::WaitingObj(arc), tf);
}

pub fn sys_sbrk(tf: &mut KernelTrapFrame) {
    let incr = tf.regs[0] as i64;
    if incr % (PAGE_SIZE as i64) != 0 {
        set_err(tf, OsError::InvalidArgument);
        return;
    }



    set_result(tf, &[0]);
    set_err(tf, OsError::Ok);
}

pub fn handle_syscall(num: u16, tf: &mut KernelTrapFrame) {

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
        NR_WAITPID => {
            let pid = tf.regs[0];
            sys_waitpid(pid, tf);
        }
        NR_WAIT_WAITABLE => {
            sys_wait_waitable(tf);
        }
        NR_SBRK => {
            sys_sbrk(tf);
        }
        _ => kprintln!("Unknown syscall: {}", num),
    }
}
