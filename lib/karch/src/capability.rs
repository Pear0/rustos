use enumset::EnumSet;
use enumset::__internal::core_export::sync::atomic::AtomicU8;

const fn can_transmute<A, B>() -> bool {
    use core::mem;
    // Sizes must be equal, but alignment of `A` must be greater or equal than that of `B`.
    mem::size_of::<A>() == mem::size_of::<B>() && mem::align_of::<A>() >= mem::align_of::<B>()
}

#[derive(EnumSetType, Debug)]
pub enum ExecCapability {
    Allocation,
    Scheduler,
}

// Assert size of ExecCapability is as expected.
const _: fn(EnumSet<ExecCapability>) -> u8 = |e| unsafe { core::mem::transmute(e) };

const _: usize = if core::mem::needs_drop::<EnumSet<ExecCapability>>() { 0usize } else { 1usize } - 1;

const _: usize = if can_transmute::<EnumSet<ExecCapability>, AtomicU8>() { 1usize } else { 0usize } - 1;

impl ExecCapability {
    pub const fn empty_set() -> EnumSet<Self> {
        enum_set!()
    }
}
