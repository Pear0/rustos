
template = '''// Auto-generated. Do not edit

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
    
{kernel_save}

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
{kernel_restore}

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
    
{hyper_save}

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
{hyper_restore}

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

'''

frames = '''// Auto-generated. Do not edit
#![allow(non_snake_case)]

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct KernelTrapFrame {{
{kernel}
    pub simd: [u128; 32],
    pub regs: [u64; 31],
    __res1: u64,
}}

const_assert_size!(KernelTrapFrame, {kernel_size});

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct HyperTrapFrame {{
{hyper}
    pub simd: [u128; 32],
    pub regs: [u64; 31],
    __res1: u64,
}}

const_assert_size!(HyperTrapFrame, {hyper_size});

'''


def chunk_pairs(item_list):
    result = []
    for i in range(0, len(item_list), 2):
        if i == len(item_list) - 1:
            result.append((item_list[i],))
        else:
            result.append((item_list[i], item_list[i + 1]))
    return result


def stack_save_system(registers):
    lines = []

    for pair in chunk_pairs(registers)[::-1]:
        if len(pair) == 2:
            lines.append('    mrs     x1, {}'.format(pair[1]))
            lines.append('    mrs     x0, {}'.format(pair[0]))
            lines.append('    stp     x0, x1, [SP, #-16]!')
            lines.append('')
        else:
            lines.append('    mrs     x0, {}'.format(pair[0]))
            lines.append('    stp     x0, xzr, [SP, #-16]!')
            lines.append('')

    return '\n'.join(lines)


def stack_restore_system(registers):
    lines = []

    for pair in chunk_pairs(registers):
        if len(pair) == 2:
            lines.append('    ldp     x0, x1, [SP], #16')
            lines.append('    msr     {}, x0'.format(pair[0]))
            lines.append('    msr     {}, x1'.format(pair[1]))
            lines.append('')
        else:
            lines.append('    ldp     x0, xzr, [SP], #16')
            lines.append('    msr     {}, x0'.format(pair[0]))
            lines.append('')

    return '\n'.join(lines)


def frame_registers(registers):
    lines = []

    for reg in registers:
        lines.append('    pub {}: u64,'.format(reg))

    if len(registers) % 2 != 0:
        lines.append('    __res0: u64,')

    return '\n'.join(lines)


def frame_size(registers):
    size = 0
    size += 16 * 32  # SIMD
    size += 8 * 31  # general registers
    size += 8  # after general regs padding

    size += 8 * len(registers)
    if len(registers) % 2 != 0:
        size += 8

    return size


def parse_regs(string):
    return string.strip().split()


kernel_registers = ['ELR_EL1', 'SPSR_EL1', 'SP_EL0', 'TPIDR_EL0', 'TTBR0_EL1', 'TTBR1_EL1']
hyper_registers = parse_regs('''
ELR_EL1
FPCR
FPSR
SP_EL0
SP_EL1
SPSR_EL1
SPSR_abt
SPSR_fiq
SPSR_irq
SPSR_und
ACTLR_EL1
AFSR0_EL1
AFSR1_EL1
AMAIR_EL1
CONTEXTIDR_EL1
CPACR_EL1
CPTR_EL2
CSSELR_EL1
ESR_EL1
FAR_EL1
MAIR_EL1
PAR_EL1
SCTLR_EL1
TCR_EL1
TPIDR_EL0
TPIDR_EL1
TPIDRRO_EL0
TTBR0_EL1
TTBR1_EL1
VBAR_EL1

CNTKCTL_EL1
CNTP_CTL_EL0
CNTP_CVAL_EL0
CNTV_CTL_EL0
CNTV_CVAL_EL0

CNTVOFF_EL2

ELR_EL2
SPSR_EL2
HCR_EL2
VTTBR_EL2
TPIDR_EL2
VMPIDR_EL2
''')


# src/traps/frame_gen.rs

with open('src/init/vectors.s', 'w') as f:
    f.write(template.format(
        kernel_save=stack_save_system(kernel_registers),
        kernel_restore=stack_restore_system(kernel_registers),
        hyper_save=stack_save_system(hyper_registers),
        hyper_restore=stack_restore_system(hyper_registers),
    ))

with open('src/traps/frame/gen.rs', 'w') as f:
    f.write(frames.format(
        kernel=frame_registers(kernel_registers),
        kernel_size=frame_size(kernel_registers),
        hyper=frame_registers(hyper_registers),
        hyper_size=frame_size(hyper_registers),
    ))
