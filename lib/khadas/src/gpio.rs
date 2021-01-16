#![allow(dead_code)]
#![allow(unused_variables)]

use volatile::Volatile;

macro_rules! const_assert_size {
    ($expr:tt, $size:tt) => {
    const _: fn(a: $expr) -> [u8; $size] = |a| unsafe { core::mem::transmute::<$expr, [u8; $size]>(a) };
    };
}

const GPIO_GROUP_SIZES: [usize; 6] = [16, 8, 20, 9, 16, 16];

#[repr(C)]
struct GpioGroup {
    enable_n: Volatile<u32>,
    output_enable: Volatile<u32>,
    input_enable: Volatile<u32>,
}

const_assert_size!(GpioGroup, 12);

#[repr(C)]
struct GpioRegisters {
    groups: [GpioGroup; 6],
}

#[repr(C)]
struct GpioPullUp {
    regs: [Volatile<u32>; 6],
}

#[repr(C)]
struct GpioPullUpEnable {
    regs: [Volatile<u32>; 6],
}

#[repr(C)]
struct GpioPeripheralMux {
    regs: [Volatile<u32>; 12],
}

pub struct Gpio {
    registers: &'static mut GpioRegisters,
    pull_up: &'static mut GpioPullUp,
    pull_up_enable: &'static mut GpioPullUpEnable,
}

impl Gpio {
    pub unsafe fn new(reg_base: u64, pull_up: u64, pull_up_enable: u64) -> Self {
        Self {
            registers: &mut *(reg_base as *mut GpioRegisters),
            pull_up: &mut *(pull_up as *mut GpioPullUp),
            pull_up_enable: &mut *(pull_up_enable as *mut GpioPullUpEnable),
        }
    }

    pub fn gpio_count() -> usize {
        GPIO_GROUP_SIZES.iter().sum()
    }

    fn idx_to_offset(mut idx: usize) -> (usize, usize) {
        for (i, size) in GPIO_GROUP_SIZES.iter().enumerate() {
            if idx < *size {
                return (i, idx);
            }
            idx -= *size;

        }
        panic!("bad index")
    }

}


