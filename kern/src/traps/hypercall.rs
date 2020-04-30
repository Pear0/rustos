use alloc::boxed::Box;
use alloc::sync::Arc;
use core::time::Duration;

use kernel_api::*;
use kernel_api::OsError;

use crate::hyper::HYPER_SCHEDULER;
use crate::net::physical::Physical;
use crate::process::{EventPollFn, HyperImpl, State, HyperProcess};
use crate::traps::{Frame, HyperTrapFrame};
use crate::vm::VirtualAddr;
use crate::net::physical;

fn set_result(tf: &mut HyperTrapFrame, regs: &[u64]) {
    for (i, v) in regs.iter().enumerate() {
        tf.regs[i] = *v;
    }
}

fn set_err(tf: &mut HyperTrapFrame, res: OsError) {
    tf.regs[7] = res as u64;
}


fn sys_sleep(ms: u32, tf: &mut HyperTrapFrame) {
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

fn sys_exit(tf: &mut HyperTrapFrame) {
    HYPER_SCHEDULER.kill(tf).expect("killed");
    // we need to schedule a new process otherwise things will be very bad
    HYPER_SCHEDULER.switch_to(tf);
}

fn net_ctx<F: FnOnce(&mut HyperTrapFrame, &mut HyperProcess) -> OsResult<()>>(tf: &mut HyperTrapFrame, func: F) {
    HYPER_SCHEDULER.crit_process(tf.get_id(), |proc| {
        let mut proc = proc.expect("proc is none");

        match func(tf, proc) {
            Ok(()) => set_err(tf, OsError::Ok),
            Err(e) => set_err(tf, e),
        }
    });
}

fn hyper_vnic_state(tf: &mut HyperTrapFrame) {
    net_ctx(tf, |tf, proc| {
        let nic = match &proc.detail.nic {
            Some(nic) => nic,
            None => return Err(OsError::InvalidSocket),
        };

        if nic.is_connected() {
            set_result(tf, &[1]);
        } else {
            set_result(tf, &[0]);
        }
        Ok(())
    });
}

fn hyper_vnic_send(tf: &mut HyperTrapFrame) {
    net_ctx(tf, |tf, proc| {
        let (addr, len) = (VirtualAddr::from(tf.regs[0]), tf.regs[1] as usize);

        if len > physical::FRAME_BUFFER_SIZE {
            return Err(OsError::InvalidArgument);
        }

        let mut frame = physical::Frame::default();
        proc.vmap.copy_out(addr, &mut frame.0[..len])?;
        frame.1 = len;

        if let Some(nic) = &proc.detail.nic {
            nic.send_frame(&frame).ok_or(OsError::IoError)?;
        } else {
            return Err(OsError::InvalidSocket);
        }

        Ok(())
    });
}

fn hyper_vnic_receive(tf: &mut HyperTrapFrame) {
    net_ctx(tf, |tf, proc| {
        let addr = VirtualAddr::from(tf.regs[0]);
        let mut frame = physical::Frame::default();

        if let Some(nic) = &proc.detail.nic {
            nic.receive_frame(&mut frame).ok_or(OsError::Waiting)?;
        } else {
            return Err(OsError::InvalidSocket);
        }

        proc.vmap.copy_in(addr, frame.as_slice())?;
        set_result(tf, &[frame.1 as u64]);
        Ok(())
    });
}


pub fn handle_hyper_syscall(num: u16, tf: &mut HyperTrapFrame) {
    set_err(tf, OsError::Unknown);
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

pub fn handle_hypercall(num: u16, tf: &mut HyperTrapFrame) {
    set_err(tf, OsError::Unknown);
    match num as usize {
        HP_VNIC_STATE => {
            hyper_vnic_state(tf);
        }
        HP_VNIC_SEND => {
            hyper_vnic_send(tf);
        }
        HP_VNIC_RECEIVE => {
            hyper_vnic_receive(tf);
        }
        _ => kprintln!("Unknown hypercall: {} @ {:#x}", num, tf.elr),
    }
}
