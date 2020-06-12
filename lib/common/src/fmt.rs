use core::fmt;

#[derive(Debug)]
pub struct ByteSize(u64);

impl From<usize> for ByteSize {
    fn from(a: usize) -> Self {
        ByteSize(a as u64)
    }
}

fn exp(n: u64, e: u64) -> u64 {
    let mut o = 1;
    for _ in 0..e {
        o *= n;
    }
    o
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffixes = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

        let mut value = self.0;
        let mut is_thing = false;
        let mut suffix = suffixes[0];

        for i in (1..suffixes.len()).rev() {
            let threshold = exp(1024, i as u64);
            let divide = exp(1024, (i - 1) as u64) * 100;

            if value >= threshold {
                value /= divide;
                is_thing = true;
                suffix = suffixes[i];
            }
        }

        if is_thing {
            f.write_fmt(format_args!("{}.{} {}", value / 10, value % 10, suffix))
        } else {
            f.write_fmt(format_args!("{} {}", value, suffix))
        }
    }
}








