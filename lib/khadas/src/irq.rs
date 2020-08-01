
use aarch64::regs::*;

/*
ref: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/plain/Documentation/devicetree/bindings/interrupt-controller/arm,gic.yaml

gic: interrupt-controller@ffc01000 {
                        compatible = "arm,gic-400";
                        reg = <0x0 0xffc01000 0 0x1000>,
                              <0x0 0xffc02000 0 0x2000>,
                              <0x0 0xffc04000 0 0x2000>,
                              <0x0 0xffc06000 0 0x2000>;
                        interrupt-controller;
                        interrupts = <GIC_PPI 9
                                (GIC_CPU_MASK_SIMPLE(8) | IRQ_TYPE_LEVEL_HIGH)>;
                        #interrupt-cells = <3>;
                        #address-cells = <0>;
                };


uart_AO: serial@3000 {
                                compatible = "amlogic,meson-gx-uart",
                                             "amlogic,meson-ao-uart";
                                reg = <0x0 0x3000 0x0 0x18>;
                                interrupts = <GIC_SPI 193 IRQ_TYPE_EDGE_RISING>;
                                clocks = <&xtal>, <&clkc_AO CLKID_AO_UART>, <&xtal>;
                                clock-names = "xtal", "pclk", "baud";
                                status = "disabled";
                        };
 */

const GICD_BASE: u64 = 0xffc0_1000;

const GICD_CTLR: *mut u32 = (GICD_BASE + 0x0) as *mut u32;

const GICD_ISENABLER0: *mut u32 = (GICD_BASE + 0x100 + 4 * 0) as *mut u32;

const GICD_PRI: *mut u32 = (GICD_BASE + 0x400 + 4 * 0) as *mut u32;

const GICD_ICFGR0: *mut u32 = (GICD_BASE + 0xC00 + 4 * 0) as *mut u32;

const GICD_IGROUPR0: *mut u32 = (GICD_BASE + 0x080 + 4 * 0) as *mut u32;

const GICC_BASE: u64 = 0xffc0_2000;

const GICC_CTLR: *mut u32 = (GICC_BASE + 0) as *mut u32;
const GICC_PMR: *mut u32 = (GICC_BASE + 4) as *mut u32;

pub fn init_stuff() {
    unsafe {
        // ICC_SRE_EL1.set(ICC_SRE_EL1.get() | 1); // Set System Register Enable bit
        // info!("ICC_SRE_EL1 => {:#032b}", ICC_SRE_EL1.get());

        // GICD_CTLR.write_volatile(GICD_CTLR::DS); // disable secure mode
        //
        // if (GICD_CTLR.read_volatile() & GICD_CTLR::DS) == 0 {
        //     info!("cannot set GICD_CTLR.DS");
        // }

        // for addr in [0xff63_c148u64, 0xff63c_0c8u64].iter() {
        //     let addr = (*addr) as *mut u32;
        //     info!("Addr: {:#x} => {:#032b}", addr as u64, addr.read_volatile());
        //     addr.write_volatile(0xffff_ffff);
        //     info!("Addr: {:#x} => {:#032b}", addr as u64, addr.read_volatile());
        // }

        // 27 = virtual timer INT ID

        for i in 0..64 {
            GICD_ICFGR0.offset(i).write_volatile(0); // grape-shot, set INTID=30 to level triggered
        }

        for i in 0..32 {
            GICD_IGROUPR0.offset(i).write_volatile(0xffff_ffff); // put physical timer in non-secure group 1
        }

        // enable virtual timer
        GICD_ISENABLER0.write_volatile(0xffff_ffff);


        GICD_CTLR.write_volatile(0b11); // enable group0 and group1ns

        GICC_PMR.write_volatile(0xf0); // set this-core interrupt mast to accept all

        GICC_CTLR.write_volatile(0b1); // enable group 1 interrupt delivery to this-core.

        // CNTV_TVAL_EL0.set(0);
        // CNTV_CTL_EL0.set((CNTV_CTL_EL0.get() & !CNTV_CTL_EL0::IMASK) | CNTV_CTL_EL0::ENABLE);
        //
        // DAIF.set(DAIF::D | DAIF::A | DAIF::F);
        //
        // aarch64::dsb();
        //
        // for i in 0..10_000_000 {
        //     GICC_CTLR.read_volatile();
        // }


    }
}






