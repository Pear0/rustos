// Auto-generated. Do not edit

.global kernel_context_save
kernel_context_save:
    stp     x26, x27, [SP, #-16]!
    stp     x24, x25, [SP, #-16]!
    stp     x22, x23, [SP, #-16]!
    stp     x20, x21, [SP, #-16]!
    stp     x18, x19, [SP, #-16]!
    stp     x16, x17, [SP, #-16]!
    stp     x14, x15, [SP, #-16]!
    stp     x12, x13, [SP, #-16]!
    stp     x10, x11, [SP, #-16]!
    stp     x8, x9, [SP, #-16]!
    stp     x6, x7, [SP, #-16]!
    stp     x4, x5, [SP, #-16]!
    stp     x2, x3, [SP, #-16]!
    stp     x0, x1, [SP, #-16]!

    stp     q30, q31, [SP, #-32]!
    stp     q28, q29, [SP, #-32]!
    stp     q26, q27, [SP, #-32]!
    stp     q24, q25, [SP, #-32]!
    stp     q22, q23, [SP, #-32]!
    stp     q20, q21, [SP, #-32]!
    stp     q18, q19, [SP, #-32]!
    stp     q16, q17, [SP, #-32]!
    stp     q14, q15, [SP, #-32]!
    stp     q12, q13, [SP, #-32]!
    stp     q10, q11, [SP, #-32]!
    stp     q8, q9, [SP, #-32]!
    stp     q6, q7, [SP, #-32]!
    stp     q4, q5, [SP, #-32]!
    stp     q2, q3, [SP, #-32]!
    stp     q0, q1, [SP, #-32]!
    
    mrs     x1, TTBR1_EL1
    mrs     x0, TTBR0_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, TPIDR_EL0
    mrs     x0, SP_EL0
    stp     x0, x1, [SP, #-16]!

    mrs     x1, SPSR_EL1
    mrs     x0, ELR_EL1
    stp     x0, x1, [SP, #-16]!


    // Set up arguments to exception handler
    mov x2, sp
    mov x0, x29
    mrs x1, ESR_EL1

    // Save our link register because we need to return here
    stp xzr, lr, [SP, #-16]!

    bl kernel_handle_exception

    ldp xzr, lr, [SP], #16

    b kernel_context_restore


.global kernel_context_restore
kernel_context_restore:
    ldp     x0, x1, [SP], #16
    msr     ELR_EL1, x0
    msr     SPSR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     SP_EL0, x0
    msr     TPIDR_EL0, x1

    ldp     x0, x1, [SP], #16
    msr     TTBR0_EL1, x0
    msr     TTBR1_EL1, x1


    // reload page tables
    dsb     ishst
    tlbi    vmalle1is
    dsb     ish
    isb

    ldp     q0, q1, [SP], #32
    ldp     q2, q3, [SP], #32
    ldp     q4, q5, [SP], #32
    ldp     q6, q7, [SP], #32
    ldp     q8, q9, [SP], #32
    ldp     q10, q11, [SP], #32
    ldp     q12, q13, [SP], #32
    ldp     q14, q15, [SP], #32
    ldp     q16, q17, [SP], #32
    ldp     q18, q19, [SP], #32
    ldp     q20, q21, [SP], #32
    ldp     q22, q23, [SP], #32
    ldp     q24, q25, [SP], #32
    ldp     q26, q27, [SP], #32
    ldp     q28, q29, [SP], #32
    ldp     q30, q31, [SP], #32

    ldp     x0, x1, [SP], #16
    ldp     x2, x3, [SP], #16
    ldp     x4, x5, [SP], #16
    ldp     x6, x7, [SP], #16
    ldp     x8, x9, [SP], #16
    ldp     x10, x11, [SP], #16
    ldp     x12, x13, [SP], #16
    ldp     x14, x15, [SP], #16
    ldp     x16, x17, [SP], #16
    ldp     x18, x19, [SP], #16
    ldp     x20, x21, [SP], #16
    ldp     x22, x23, [SP], #16
    ldp     x24, x25, [SP], #16
    ldp     x26, x27, [SP], #16

    ret


.macro KERNEL_HANDLER source, kind
    .align 7
    stp     lr, xzr, [SP, #-16]!
    stp     x28, x29, [SP, #-16]!
    
    mov     x29, \source
    movk    x29, \kind, LSL #16
    bl      kernel_context_save
    
    ldp     x28, x29, [SP], #16
    ldp     lr, xzr, [SP], #16
    eret
.endm
    
.align 11
.global kernel_vectors
kernel_vectors:
    KERNEL_HANDLER 0, 0
    KERNEL_HANDLER 0, 1
    KERNEL_HANDLER 0, 2
    KERNEL_HANDLER 0, 3

    KERNEL_HANDLER 1, 0
    KERNEL_HANDLER 1, 1
    KERNEL_HANDLER 1, 2
    KERNEL_HANDLER 1, 3

    KERNEL_HANDLER 2, 0
    KERNEL_HANDLER 2, 1
    KERNEL_HANDLER 2, 2
    KERNEL_HANDLER 2, 3

    KERNEL_HANDLER 3, 0
    KERNEL_HANDLER 3, 1
    KERNEL_HANDLER 3, 2
    KERNEL_HANDLER 3, 3



.global hyper_context_save
hyper_context_save:
    stp     x26, x27, [SP, #-16]!
    stp     x24, x25, [SP, #-16]!
    stp     x22, x23, [SP, #-16]!
    stp     x20, x21, [SP, #-16]!
    stp     x18, x19, [SP, #-16]!
    stp     x16, x17, [SP, #-16]!
    stp     x14, x15, [SP, #-16]!
    stp     x12, x13, [SP, #-16]!
    stp     x10, x11, [SP, #-16]!
    stp     x8, x9, [SP, #-16]!
    stp     x6, x7, [SP, #-16]!
    stp     x4, x5, [SP, #-16]!
    stp     x2, x3, [SP, #-16]!
    stp     x0, x1, [SP, #-16]!

    stp     q30, q31, [SP, #-32]!
    stp     q28, q29, [SP, #-32]!
    stp     q26, q27, [SP, #-32]!
    stp     q24, q25, [SP, #-32]!
    stp     q22, q23, [SP, #-32]!
    stp     q20, q21, [SP, #-32]!
    stp     q18, q19, [SP, #-32]!
    stp     q16, q17, [SP, #-32]!
    stp     q14, q15, [SP, #-32]!
    stp     q12, q13, [SP, #-32]!
    stp     q10, q11, [SP, #-32]!
    stp     q8, q9, [SP, #-32]!
    stp     q6, q7, [SP, #-32]!
    stp     q4, q5, [SP, #-32]!
    stp     q2, q3, [SP, #-32]!
    stp     q0, q1, [SP, #-32]!
    
    mrs     x1, VMPIDR_EL2
    mrs     x0, TPIDR_EL2
    stp     x0, x1, [SP, #-16]!

    mrs     x1, VTTBR_EL2
    mrs     x0, HCR_EL2
    stp     x0, x1, [SP, #-16]!

    mrs     x1, SPSR_EL2
    mrs     x0, ELR_EL2
    stp     x0, x1, [SP, #-16]!

    mrs     x1, CNTVOFF_EL2
    mrs     x0, CNTV_CVAL_EL0
    stp     x0, x1, [SP, #-16]!

    mrs     x1, CNTV_CTL_EL0
    mrs     x0, CNTP_CVAL_EL0
    stp     x0, x1, [SP, #-16]!

    mrs     x1, CNTP_CTL_EL0
    mrs     x0, CNTKCTL_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, VBAR_EL1
    mrs     x0, TTBR1_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, TTBR0_EL1
    mrs     x0, TPIDRRO_EL0
    stp     x0, x1, [SP, #-16]!

    mrs     x1, TPIDR_EL1
    mrs     x0, TPIDR_EL0
    stp     x0, x1, [SP, #-16]!

    mrs     x1, TCR_EL1
    mrs     x0, SCTLR_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, PAR_EL1
    mrs     x0, MAIR_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, FAR_EL1
    mrs     x0, ESR_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, CSSELR_EL1
    mrs     x0, CPTR_EL2
    stp     x0, x1, [SP, #-16]!

    mrs     x1, CPACR_EL1
    mrs     x0, CONTEXTIDR_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, AMAIR_EL1
    mrs     x0, AFSR1_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, AFSR0_EL1
    mrs     x0, ACTLR_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, SPSR_und
    mrs     x0, SPSR_irq
    stp     x0, x1, [SP, #-16]!

    mrs     x1, SPSR_fiq
    mrs     x0, SPSR_abt
    stp     x0, x1, [SP, #-16]!

    mrs     x1, SPSR_EL1
    mrs     x0, SP_EL1
    stp     x0, x1, [SP, #-16]!

    mrs     x1, SP_EL0
    mrs     x0, FPSR
    stp     x0, x1, [SP, #-16]!

    mrs     x1, FPCR
    mrs     x0, ELR_EL1
    stp     x0, x1, [SP, #-16]!


    // Set up arguments to exception handler
    mov x2, sp
    mov x0, x29
    mrs x1, ESR_EL2

    // Save our link register because we need to return here
    stp xzr, lr, [SP, #-16]!

    bl hyper_handle_exception

    ldp xzr, lr, [SP], #16

    b hyper_context_restore


.global hyper_context_restore
hyper_context_restore:
    ldp     x0, x1, [SP], #16
    msr     ELR_EL1, x0
    msr     FPCR, x1

    ldp     x0, x1, [SP], #16
    msr     FPSR, x0
    msr     SP_EL0, x1

    ldp     x0, x1, [SP], #16
    msr     SP_EL1, x0
    msr     SPSR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     SPSR_abt, x0
    msr     SPSR_fiq, x1

    ldp     x0, x1, [SP], #16
    msr     SPSR_irq, x0
    msr     SPSR_und, x1

    ldp     x0, x1, [SP], #16
    msr     ACTLR_EL1, x0
    msr     AFSR0_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     AFSR1_EL1, x0
    msr     AMAIR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     CONTEXTIDR_EL1, x0
    msr     CPACR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     CPTR_EL2, x0
    msr     CSSELR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     ESR_EL1, x0
    msr     FAR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     MAIR_EL1, x0
    msr     PAR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     SCTLR_EL1, x0
    msr     TCR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     TPIDR_EL0, x0
    msr     TPIDR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     TPIDRRO_EL0, x0
    msr     TTBR0_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     TTBR1_EL1, x0
    msr     VBAR_EL1, x1

    ldp     x0, x1, [SP], #16
    msr     CNTKCTL_EL1, x0
    msr     CNTP_CTL_EL0, x1

    ldp     x0, x1, [SP], #16
    msr     CNTP_CVAL_EL0, x0
    msr     CNTV_CTL_EL0, x1

    ldp     x0, x1, [SP], #16
    msr     CNTV_CVAL_EL0, x0
    msr     CNTVOFF_EL2, x1

    ldp     x0, x1, [SP], #16
    msr     ELR_EL2, x0
    msr     SPSR_EL2, x1

    ldp     x0, x1, [SP], #16
    msr     HCR_EL2, x0
    msr     VTTBR_EL2, x1

    ldp     x0, x1, [SP], #16
    msr     TPIDR_EL2, x0
    msr     VMPIDR_EL2, x1


    // reload page tables
    dsb     sy
    tlbi    vmalls12e1is
    tlbi    alle2is
    dsb     sy
    isb

    // ic iallu
    // isb

    ldp     q0, q1, [SP], #32
    ldp     q2, q3, [SP], #32
    ldp     q4, q5, [SP], #32
    ldp     q6, q7, [SP], #32
    ldp     q8, q9, [SP], #32
    ldp     q10, q11, [SP], #32
    ldp     q12, q13, [SP], #32
    ldp     q14, q15, [SP], #32
    ldp     q16, q17, [SP], #32
    ldp     q18, q19, [SP], #32
    ldp     q20, q21, [SP], #32
    ldp     q22, q23, [SP], #32
    ldp     q24, q25, [SP], #32
    ldp     q26, q27, [SP], #32
    ldp     q28, q29, [SP], #32
    ldp     q30, q31, [SP], #32

    ldp     x0, x1, [SP], #16
    ldp     x2, x3, [SP], #16
    ldp     x4, x5, [SP], #16
    ldp     x6, x7, [SP], #16
    ldp     x8, x9, [SP], #16
    ldp     x10, x11, [SP], #16
    ldp     x12, x13, [SP], #16
    ldp     x14, x15, [SP], #16
    ldp     x16, x17, [SP], #16
    ldp     x18, x19, [SP], #16
    ldp     x20, x21, [SP], #16
    ldp     x22, x23, [SP], #16
    ldp     x24, x25, [SP], #16
    ldp     x26, x27, [SP], #16

    ret


.macro HYPER_HANDLER source, kind
    .align 7
    stp     lr, xzr, [SP, #-16]!
    stp     x28, x29, [SP, #-16]!

    mov     x29, \source
    movk    x29, \kind, LSL #16
    bl      hyper_context_save

    ldp     x28, x29, [SP], #16
    ldp     lr, xzr, [SP], #16
    eret
.endm

.align 11
.global hyper_vectors
hyper_vectors:
    HYPER_HANDLER 0, 0
    HYPER_HANDLER 0, 1
    HYPER_HANDLER 0, 2
    HYPER_HANDLER 0, 3

    HYPER_HANDLER 1, 0
    HYPER_HANDLER 1, 1
    HYPER_HANDLER 1, 2
    HYPER_HANDLER 1, 3

    HYPER_HANDLER 2, 0
    HYPER_HANDLER 2, 1
    HYPER_HANDLER 2, 2
    HYPER_HANDLER 2, 3

    HYPER_HANDLER 3, 0
    HYPER_HANDLER 3, 1
    HYPER_HANDLER 3, 2
    HYPER_HANDLER 3, 3

