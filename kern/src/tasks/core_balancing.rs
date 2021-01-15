use alloc::vec::Vec;
use core::time::Duration;

use crate::kernel::{KERNEL_CORES, KERNEL_SCHEDULER};
use crate::process::KernProcessCtx;
use crate::smp::{core_bootstrap, with_each_core};
use kscheduler::{Scheduler, Process};
use hashbrown::HashMap;

#[derive(Debug, Default)]
struct CoreInfo {
    total_load: usize,
}

pub fn core_balancing_thread(ctx: KernProcessCtx) {
    loop {
        let mut snaps = Vec::new();
        KERNEL_SCHEDULER.get_all_process_snaps(&mut snaps);
        let mut new_assignment: Vec<_> = snaps.iter().map(|x| x.core as usize).collect();

        let mut cores = Vec::new();
        cores.resize_with(*KERNEL_CORES, || CoreInfo::default());

        for snap in &snaps {
            // we don't count idle process in load
            if snap.tpidr == 0 {
                continue;
            }

            cores[snap.core as usize].total_load += snap.cpu_usage as usize;
        }

        info!("core loads: {:?}", cores.iter().map(|x| x.total_load).collect::<Vec<_>>());

        for (idx, snap) in snaps.iter().enumerate() {
            if snap.tpidr == 0 {
                continue;
            }

            for core_id in 0..*KERNEL_CORES {
                if snap.affinity.check(core_id) && core_id != new_assignment[idx] {
                    let current_core_load = cores[new_assignment[idx]].total_load - snap.cpu_usage as usize;
                    if cores[core_id].total_load < current_core_load {

                        // Do re-assignment
                        info!("moving pid={} core {} -> {}", snap.tpidr, new_assignment[idx], core_id);
                        cores[core_id].total_load += snap.cpu_usage as usize;
                        cores[new_assignment[idx]].total_load -= snap.cpu_usage as usize;
                        new_assignment[idx] = core_id;
                        break;
                    }
                }
            }
        }

        // pid -> core_id
        let mut lookup = HashMap::<u64, usize>::new();
        for (snap, core_id) in snaps.iter().zip(new_assignment.iter().cloned()) {
            if snap.tpidr == 0 {
                continue;
            }
            lookup.insert(snap.tpidr, core_id);
        }

        KERNEL_SCHEDULER.iter_all_processes(|core_id, proc| {
            if let Some(new_core_id) = lookup.get(&(proc.get_id() as u64)).cloned() {
                if core_id != new_core_id {
                    proc.set_send_to_core(Some(new_core_id));
                }
            }
        });

        kernel_api::syscall::sleep(Duration::from_secs(3));
    }
}

