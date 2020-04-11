#![feature(asm)]
#![no_std]
#![no_main]

mod cr0;

use kernel_api::println;
use kernel_api::syscall::{getpid, time, sleep};
use core::time::Duration;

fn main() {
    let pid = getpid();

    for i in 1..=60 {
        println!("wait pid={}, time={}s", pid, i);
        sleep(Duration::from_secs(1));
    }

    println!("wait pid={}, done!", pid);
}
