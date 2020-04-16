mod regs {
    // Control and Status register
    defbit32!(DMA_CS, [




        Aff3 [39-32], // Affinity level 3
        U    [30-30], // Indicates a Uniprocessor system
        MT   [24-24], // Multithreading type approach
        Aff2 [23-16], // Affinity level 2
        Aff1 [15-08], // Affinity level 1
        Aff0 [07-00], // Affinity level 0

        RES0 [63-40|29-25],
        RES1 [31-31],
    ]);
}

use regs::*;

fn foo() {
    use regs::*;

    DMA_CS::new()


}

/// a struct matching the BCM 2835 DMA Control Block structure.
#[repr(C, align(32))]
struct InnerBlock {
    cs: DMA_CS,

}

/// A wrapper type that is used by Rust code to more ergonomically configure
/// DMA transfers and
#[derive(Clone, Debug)]
pub struct ControlBlock {}