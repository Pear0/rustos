use core::fmt;

#[derive(Copy, Clone, Default, Debug)]
pub struct Timestamp {
    // TODO do not make these public, maybe switch to UNIX epoch or similar
    year: u32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl Timestamp {

    pub fn new_from_fields(year: u32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self { year, month, day, hour, minute, second }
    }

    /// The calendar year.
    ///
    /// The year is not offset. 2009 is 2009.
    pub fn year(&self) -> usize {
        self.year as usize
    }

    /// The calendar month, starting at 1 for January. Always in range [1, 12].
    ///
    /// January is 1, Feburary is 2, ..., December is 12.
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The calendar day, starting at 1. Always in range [1, 31].
    pub fn day(&self) -> u8 {
        self.day
    }

    /// The 24-hour hour. Always in range [0, 24).
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute. Always in range [0, 60).
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The second. Always in range [0, 60).
    pub fn second(&self) -> u8 {
        self.second
    }
}

#[derive(Clone, Default, Debug)]
pub struct Metadata {
    pub read_only: Option<bool>,
    pub hidden: Option<bool>,
    pub created: Option<Timestamp>,
    pub accessed: Option<Timestamp>,
    pub modified: Option<Timestamp>,
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}", self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second()))
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metadata")
            .field("read_only", &self.read_only)
            .field("hidden", &self.hidden)
            .field("created", &self.created)
            .field("modified", &self.modified)
            .field("accessed", &self.accessed)
            .finish()
    }
}


