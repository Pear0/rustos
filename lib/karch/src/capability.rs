use enumset::EnumSet;

#[derive(EnumSetType, Debug)]
pub enum ExecCapability {
    Allocation,
    Scheduler,
}

// Assert size of ExecCapability is as expected.
const _: fn(EnumSet<ExecCapability>) -> u8 = |e| unsafe { core::mem::transmute(e) };

const _: usize = if core::mem::needs_drop::<EnumSet<ExecCapability>>() { 0usize } else { 1usize } - 1;

impl ExecCapability {
    pub const fn empty_set() -> EnumSet<Self> {
        enum_set!()
    }
}
