use crate::common::*;

pub unsafe fn reset() -> ! {
    let r = PM_RSTS.read_volatile() & !0xfffffaaa;

    PM_RSTS.write_volatile(PM_WDOG_MAGIC | r);
    PM_WDOG.write_volatile(PM_WDOG_MAGIC | 10);
    PM_RSTC.write_volatile(PM_WDOG_MAGIC | PM_RSTC_FULLRST);

    loop {}
}
