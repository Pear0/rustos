
// Semihosting wrappers for QEMU
// https://static.docs.arm.com/dui0003/b/semihosting.pdf

// QEMU flag: -semihosting

unsafe fn do_call(op: u32, param: u64) -> u64 {
    let mut out: u64 = 0;
    llvm_asm!("hlt #0x3C" : "={x0}"(out) : "{w0}"(op), "{x1}"(param) : "memory" : "volatile" );
    out
}







