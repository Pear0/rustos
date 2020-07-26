use alloc::vec::Vec;
use core::time::Duration;

use rand::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use crate::arm::{GenericCounterImpl, PhysicalCounter};

const NANOS_PER_SEC: u64 = 1_000_000_000;
const MICROS_PER_SEC: u64 = 1_000_000;

type PerfFn = dyn FnMut(usize);

pub fn sleep<T: GenericCounterImpl>(dur: Duration) {
    let end = time_to_cycles::<T>(clock_time::<T>() + dur);
    while end > T::get_counter() {}
}

pub fn sleep_phys(dur: Duration) {
    sleep::<PhysicalCounter>(dur)
}

pub fn clock_freq() -> u64 {
    use aarch64::regs::*;
    unsafe { CNTFRQ_EL0.get() }
}

pub fn cycles_to_time<T: GenericCounterImpl>(cycles: u64) -> Duration {
    let nanos = (NANOS_PER_SEC as u128) * (cycles as u128) / (T::get_frequency() as u128);
    Duration::from_nanos(nanos as u64)
}

pub fn time_to_cycles<T: GenericCounterImpl>(dur: Duration) -> u64 {
    let cycles = dur.as_nanos() * (T::get_frequency() as u128) / (NANOS_PER_SEC as u128);
    cycles as u64
}

pub fn clock_time<T: GenericCounterImpl>() -> Duration {
    cycles_to_time::<T>(T::get_counter())
}

// returns exec time in cycles.

fn perf_fn<T: GenericCounterImpl>(func: &mut PerfFn, count: usize) -> u64 {
    let start = T::get_counter();
    func(count);
    let end = T::get_counter();
    end - start
}

fn do_timing<T: GenericCounterImpl>(func: &mut PerfFn) -> Vec<(usize, u64)> {
    let mut readings: Vec<(usize, u64)> = Vec::new();
    let mut rnd = XorShiftRng::seed_from_u64(42);
    let mut rand_max_count = 0;
    let mut rand_mode = false;

    // 20ms
    let rand_threshold = T::get_frequency() / 50;
    let minimum_timing_threshold = T::get_frequency();

    let mut count: usize = 1;
    let mut total_timing = 0;

    {
        // make sure we have at least one zero reading
        let time = perf_fn::<T>(func, 0);
        readings.push((0, time));
    }

    loop {
        let time = perf_fn::<T>(func, count);
        readings.push((count, time));
        total_timing += time;

        if time > rand_threshold {
            rand_max_count = count;
            rand_mode = true;
        } else {
            count *= 2;
        }

        if rand_mode {
            // this is not uniform but it should be close enough.
            count = (rnd.next_u64() as usize) % rand_max_count;
        }

        if total_timing > minimum_timing_threshold && readings.len() > 20 {
            break;
        }
    }

    readings
}

// process a series of readings (count, clock cycles) and return beta of a simple
// linear regression. This code uses all integers under the assumption that beta will
// always be positive (>= 1 clock cycle per requested loop of the code under test).
fn linear_regression(readings: &Vec<(usize, u64)>) -> u64 {
    let x_bar = (readings.iter().cloned().map(|(x, _)| x as u64).sum::<u64>() / readings.len() as u64) as i128;
    let y_bar = (readings.iter().cloned().map(|(_, x)| x).sum::<u64>() / readings.len() as u64) as i128;

    let mut numerator: i128 = 0;
    let mut denominator: i128 = 0;

    for (x, y) in readings.iter() {
        let (x, y) = (*x as i128, *y as i128);
        numerator += (x - x_bar) * (y - y_bar);
        denominator += (x - x_bar) * (x - x_bar);
    }

    let ratio = numerator / denominator;
    if ratio < 0 {
        return 0;
    }

    ratio as u64
}

#[inline(never)]
pub fn benchmark_func_time(func: &mut PerfFn) -> Duration {
    let readings = do_timing::<PhysicalCounter>(func);
    let cycles = linear_regression(&readings);
    cycles_to_time::<PhysicalCounter>(cycles)
}

#[inline(never)]
pub fn benchmark_func(name: &'static str, func: &mut PerfFn) {
    debug!("[{}] Benchmarking", name);
    let readings = do_timing::<PhysicalCounter>(func);
    debug!("[{}] Got {} readings", name, readings.len());
    let cycles = linear_regression(&readings);
    if cycles == 0 {
        info!("[{}] regression beta is zero/negative, did you loop?", name)
    } else {
        info!("[{}] Benchmark: {:?}", name, cycles_to_time::<PhysicalCounter>(cycles));
    }
}

pub fn benchmark<F: FnMut(usize) + 'static>(name: &'static str, mut func: F) {
    benchmark_func(name, &mut func);
}
