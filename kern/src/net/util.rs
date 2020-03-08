use core::ops::BitOr;
use core::ops::AddAssign;
use fat32::util::SliceExt;

#[derive(Copy, Clone, Default)]
pub struct ChecksumOnesComplement {
    val: u16,
}

#[repr(align(2))]
struct ToyAlign<T> {
    phantom: u16,
    value: T,
}

impl ChecksumOnesComplement {
    pub fn new() -> Self {
        Self { val: 0 }
    }

    pub fn get(&self) -> u16 {
        if self.val == 0 || self.val == 0xFF_FF {
            0xFF_FF
        } else {
            u16::from_be(!self.val)
        }
    }

    pub fn ingest_sized<T>(&mut self, value: &T) where T : Sized + Clone {
        assert_eq!(core::mem::size_of::<T>() % 2, 0);

        let toy = ToyAlign { phantom: 0, value: value.clone() };

        let values: &[u16] = unsafe { core::slice::from_raw_parts(
            (&toy.value) as *const T as usize as *const u16, core::mem::size_of::<T>() / 2) };
        for value in values.into_iter() {
            *self += *value;
        }
    }

    pub fn ingest_u8_pad(&mut self, value: &[u8]) {
        for a in value.chunks(2).into_iter() {
            // little endian
            let num: u16;
            if a.len() == 2 {
                num = ((a[0] as u16) << 0) | ((a[1] as u16) << 8);
            } else {
                num = ((a[0] as u16) << 0);
            }
            *self += num;
        }
    }

    pub fn digest_sized<T>(value: &T) -> u16 where T : Sized + Clone {
        let mut d = Self::new();
        d.ingest_sized(value);
        d.get()
    }

}

impl AddAssign<u16> for ChecksumOnesComplement {
    fn add_assign(&mut self, rhs: u16) {
        let inputs_both_zero = (self.val | rhs) == From::from(0u8);
        let temp = self.val.wrapping_add(rhs);
        self.val = temp.wrapping_add({ if temp < self.val {From::from(1u8)} else {From::from(0u8)} });
        debug_assert!((self.val != From::from(0u8)) | inputs_both_zero);
    }
}
