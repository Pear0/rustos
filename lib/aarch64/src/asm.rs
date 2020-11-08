/// Wait for event not to burn CPU.
#[inline(always)]
pub fn wfe() {
    unsafe { llvm_asm!("wfe" :::: "volatile") };
}

/// Wait for interrupt not to burn CPU.
#[inline(always)]
pub fn wfi() {
    unsafe { llvm_asm!("wfi" :::: "volatile") };
}

pub fn halt_loop() -> ! {
    loop {
        wfi();
    }
}

/// A NOOP that won't be optimized out.
#[inline(always)]
pub fn nop() {
    unsafe { llvm_asm!("nop" :::: "volatile") };
}

/// Data Memory Barrier (ref. B2.3.5) (ref. C6.2.65)
#[inline(always)]
pub fn dmb() {
    unsafe { llvm_asm!("dmb SY" ::: "memory" : "volatile") };
}

/// Data Synchronization Barrier (ref. B2.3.5) (ref. C6.2.67)
#[inline(always)]
pub fn dsb() {
    unsafe { llvm_asm!("dsb SY" ::: "memory" : "volatile") };
}

/// Transition to a lower level
#[inline(always)]
pub fn eret() {
    unsafe { llvm_asm!("eret" :::: "volatile") };
}

/// Instruction Synchronization Barrier
#[inline(always)]
pub fn isb() {
    unsafe { llvm_asm!("isb" :::: "volatile") };
}

/// Set Event
#[inline(always)]
pub fn sev() {
    unsafe { llvm_asm!("sev" ::::"volatile") };
}

/// Set Event Local
#[inline(always)]
pub fn sevl() {
    unsafe { llvm_asm!("sevl" ::::"volatile") };
}

/// Enable (unmask) interrupts
#[inline(always)]
pub unsafe fn sti() {
    llvm_asm!("msr DAIFClr, 0b0010"
         :
         :
         :
         : "volatile");
}

/// Disable (mask) interrupt
#[inline(always)]
pub unsafe fn cli() {
    llvm_asm!("msr DAIFSet, 0b0010"
         :
         :
         :
         : "volatile");
}

/// Break with an immeidate
#[macro_export]
macro_rules! brk {
    ($num:tt) => {
        unsafe { llvm_asm!(concat!("brk ", stringify!($num)) :::: "volatile"); }
    }
}

/// Supervisor call with an immediate
#[macro_export]
macro_rules! svc {
    ($num:tt) => {
        unsafe { llvm_asm!(concat!("svc ", stringify!($num)) :::: "volatile"); }
    }
}

/// Hypervisor call with an immediate
#[macro_export]
macro_rules! hvc {
    ($num:tt) => {
        unsafe { llvm_asm!(concat!("hvc ", stringify!($num)) :::: "volatile"); }
    }
}

