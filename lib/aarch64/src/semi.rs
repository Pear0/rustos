
// Semihosting wrappers for QEMU
// https://static.docs.arm.com/dui0003/b/semihosting.pdf

// QEMU flag: -semihosting

unsafe fn do_call(op: u32, param: u64) -> u64 {
    let mut out: u64 = 0;
    llvm_asm!("hlt #0xF000" : "={x0}"(out) : "{w0}"(op), "{x1}"(param) : "memory" : "volatile" );
    out
}

pub fn sys_writec(u: u8) {
    let u: u64 = u as u64;
    unsafe { do_call(0x03, (&u) as *const _ as u64) };
}

pub fn sys_time() -> u64 {
    unsafe { do_call(0x11, 0) }
}



