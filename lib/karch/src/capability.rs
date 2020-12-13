use enumset::EnumSet;

#[derive(EnumSetType, Debug)]
pub enum ExecCapability {
    Allocation,
    Scheduler,
}

impl ExecCapability {
    pub const fn empty_set() -> EnumSet<Self> {
        enum_set!()
    }
}
