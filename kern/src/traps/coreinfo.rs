use alloc::vec::Vec;
use core::time::Duration;

use hashbrown::HashMap;

use fat32::vfat::Time;

use crate::cls::{CoreLocal, CoreMutex};
use crate::iosync::Global;
use crate::mutex::Mutex;
use crate::process::TimeRatio;
use crate::smp;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ExceptionType {
    Unknown,
    Irq,
    DataAccess(u64),
}

type Stats = (Duration, usize);

static EXC_RATIO: CoreLocal<Global<TimeRatio>> = CoreLocal::new_global(|| TimeRatio::new());
static EXC_TIME: CoreLocal<Global<HashMap<ExceptionType, Stats>>> = CoreLocal::new_global(|| HashMap::new());

pub fn exc_enter() {
    EXC_RATIO.critical(|r| r.set_active(true));
}

pub fn exc_exit() {
    EXC_RATIO.critical(|r| r.set_active(false));
}

pub fn exc_record_time(t: ExceptionType, time: Duration) {
    EXC_TIME.critical(|m| {
        let mut before = m.get(&t).map(|x| *x).unwrap_or((Duration::default(), 0));
        before.0 += time;
        before.1 += 1;
        m.insert(t, before);
    });
}

pub fn exc_ratio() -> [(TimeRatio, Vec<(ExceptionType, Stats)>); 4] {
    smp::no_interrupt(|| {
        let mut ratios = [(TimeRatio::new(), Vec::new()), (TimeRatio::new(), Vec::new()), (TimeRatio::new(), Vec::new()), (TimeRatio::new(), Vec::new())];
        for i in 0..4 {
            let ratio = unsafe { EXC_RATIO.cross(i).critical(|r| r.clone()) };

            let items = unsafe {
                EXC_TIME.cross(i).critical(|r| {
                    let mut items = Vec::new();

                    for (k, v) in r.iter() {
                        items.push((*k, *v));
                    }

                    items.sort_by_key(|(_, v)| Duration::from_secs(1000000000) - (v.0));

                    items
                })
            };

            ratios[i] = (ratio, items);
        }
        ratios
    })
}

