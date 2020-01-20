// FIXME: Make me pass! Diff budget: 25 lines.

#[derive(Debug, Eq)]
enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16),
}

// What traits does `Duration` need to implement?
fn coerce(dur: &Duration) -> u64 {
    match dur {
        Duration::MilliSeconds(s) => *s,
        Duration::Seconds(s) => (*s as u64) * 1000,
        Duration::Minutes(s) => (*s as u64) * 1000 * 60,
    }
}

impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        coerce(self) == coerce(other)
    }
}

#[test]
fn traits() {
    assert_eq!(Duration::Seconds(120), Duration::Minutes(2));
    assert_eq!(Duration::Seconds(420), Duration::Minutes(7));
    assert_eq!(Duration::MilliSeconds(420000), Duration::Minutes(7));
    assert_eq!(Duration::MilliSeconds(43000), Duration::Seconds(43));
}
