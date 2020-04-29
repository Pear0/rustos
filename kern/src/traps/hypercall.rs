use crate::traps::HyperTrapFrame;
use kernel_api::OsError;
use crate::hyper::HYPER_SCHEDULER;
use crate::process::{HyperImpl, State, EventPollFn};
use core::time::Duration;
use kernel_api::*;
use alloc::boxed::Box;

fn set_result(tf: &mut HyperTrapFrame, regs: &[u64]) {
    for (i, v) in regs.iter().enumerate() {
        tf.regs[i] = *v;
    }
}

fn set_err(tf: &mut HyperTrapFrame, res: OsError) {
    tf.regs[7] = res as u64;
}



pub fn sys_sleep(ms: u32, tf: &mut HyperTrapFrame) {
    if ms == 0 {
        HYPER_SCHEDULER.switch(State::Ready, tf);
    } else {
        let start = pi::timer::current_time();
        let wait_until = start + Duration::from_millis(ms as u64);

        let time_fn: EventPollFn<HyperImpl> = Box::new(move |tf| {
            let now = pi::timer::current_time();
            let good = now >= wait_until;
            if good {
                let d = (now - start).as_millis() as u64;
                set_result(&mut tf.context, &[d]);
                set_err(&mut tf.context, OsError::Ok);
            }
            good
        });
        HYPER_SCHEDULER.switch(State::Waiting(time_fn), tf);
    }
}

pub fn sys_exit(tf: &mut HyperTrapFrame) {
    HYPER_SCHEDULER.kill(tf).expect("killed");
    // we need to schedule a new process otherwise things will be very bad
    HYPER_SCHEDULER.switch_to(tf);
}



pub fn handle_hyper_syscall(num: u16, tf: &mut HyperTrapFrame) {
    match num as usize {
        NR_SLEEP => {
            let time = tf.regs[0];
            sys_sleep(time as u32, tf);
        }
        NR_EXIT => {
            sys_exit(tf);
        }
        _ => kprintln!("Unknown syscall in an EL2 context: {}", num),
    }
}


