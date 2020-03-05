use core::fmt;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Address([u8; 4]);

impl From<&[u8]> for Address {
    fn from(buf: &[u8]) -> Self {
        assert_eq!(buf.len(), 4);
        let mut addr = Address([0; 4]);
        addr.0.copy_from_slice(buf);
        addr
    }
}

impl From<&[u8; 4]> for Address {
    fn from(buf: &[u8; 4]) -> Self {
        assert_eq!(buf.len(), 4);
        let mut addr = Address([0; 4]);
        addr.0.copy_from_slice(buf);
        addr
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}.{}.{}.{}",
                                 self.0[0], self.0[1], self.0[2], self.0[3]))
    }
}


