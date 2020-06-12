pub struct _SP;
impl _SP {
    /// Returns the current stack pointer.
    #[inline(always)]
    pub fn get(&self) -> usize {
        let mut rtn: usize = 0;
        unsafe {
            llvm_asm!("mov $0, sp": "=r"(rtn) ::: "volatile");
        }
        rtn
    }

    /// Set the current stack pointer with an passed argument.
    #[inline(always)]
    pub unsafe fn set(&self, stack: usize) {
        llvm_asm!("mov sp, $0":: "r"(stack) :: "volatile");
    }
}
pub static SP: _SP = _SP {};

pub struct _LR;
impl _LR {
    /// Returns the current stack pointer.
    #[inline(always)]
    pub fn get(&self) -> usize {
        let mut rtn: usize = 0;
        unsafe {
            // llvm_asm!("mov $0, lr": "=r"(rtn) ::: "volatile");
            llvm_asm!("": "={lr}"(rtn) ::: "volatile");
        }
        rtn
    }

    /// Set the current stack pointer with an passed argument.
    #[inline(always)]
    pub unsafe fn set(&self, stack: usize) {
        llvm_asm!("mov lr, $0":: "r"(stack) :: "volatile");
    }
}
pub static LR: _LR = _LR {};