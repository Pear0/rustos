#![allow(unused_macros)]

#[macro_export]
macro_rules! do_hypercall0 {
    ($sys:expr) => ({
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys)
            : "memory"
            : "volatile" );
        ()
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        ()
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        ()
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        ()
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        ()
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        ()
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        ()
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        llvm_asm!(
            "hvc $0"
            : 
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        ()
    });
}
#[macro_export]
macro_rules! do_hypercall1 {
    ($sys:expr) => ({
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys)
            : "memory"
            : "volatile" );
        (o0)
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        (o0)
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        (o0)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        (o0)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        (o0)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        (o0)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        (o0)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64;
        llvm_asm!(
            "hvc $1"
            : "={x0}"(o0)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        (o0)
    });
}
#[macro_export]
macro_rules! do_hypercall2 {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x1}"(o1)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        (o0, o1)
    });
}
#[macro_export]
macro_rules! do_hypercall3 {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        (o0, o1, o2)
    });
}
#[macro_export]
macro_rules! do_hypercall4 {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3)
    });
}
#[macro_export]
macro_rules! do_hypercall5 {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4)
    });
}
#[macro_export]
macro_rules! do_hypercall6 {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5)
    });
}
#[macro_export]
macro_rules! do_hypercall7 {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        (o0, o1, o2, o3, o4, o5, o6)
    });
}
#[macro_export]
macro_rules! do_hypercall0r {
    ($sys:expr) => ({
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut ecode: u64;
        llvm_asm!(
            "hvc $1"
            : "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, ())
    });
}
#[macro_export]
macro_rules! do_hypercall1r {
    ($sys:expr) => ({
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $2"
            : "={x0}"(o0), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0))
    });
}
#[macro_export]
macro_rules! do_hypercall2r {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $3"
            : "={x0}"(o0), "={x1}"(o1), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1))
    });
}
#[macro_export]
macro_rules! do_hypercall3r {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $4"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2))
    });
}
#[macro_export]
macro_rules! do_hypercall4r {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $5"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3))
    });
}
#[macro_export]
macro_rules! do_hypercall5r {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $6"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4))
    });
}
#[macro_export]
macro_rules! do_hypercall6r {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $7"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5))
    });
}
#[macro_export]
macro_rules! do_hypercall7r {
    ($sys:expr) => ({
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
    ($sys:expr,$i0:expr) => ({
        let (i0): (u64) = ($i0);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
    ($sys:expr,$i0:expr,$i1:expr) => ({
        let (i0, i1): (u64, u64) = ($i0, $i1);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr) => ({
        let (i0, i1, i2): (u64, u64, u64) = ($i0, $i1, $i2);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr) => ({
        let (i0, i1, i2, i3): (u64, u64, u64, u64) = ($i0, $i1, $i2, $i3);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr) => ({
        let (i0, i1, i2, i3, i4): (u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr) => ({
        let (i0, i1, i2, i3, i4, i5): (u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
    ($sys:expr,$i0:expr,$i1:expr,$i2:expr,$i3:expr,$i4:expr,$i5:expr,$i6:expr) => ({
        let (i0, i1, i2, i3, i4, i5, i6): (u64, u64, u64, u64, u64, u64, u64) = ($i0, $i1, $i2, $i3, $i4, $i5, $i6);
        let mut o0: u64; let mut o1: u64; let mut o2: u64; let mut o3: u64; let mut o4: u64; let mut o5: u64; let mut o6: u64;
        let mut ecode: u64;
        llvm_asm!(
            "hvc $8"
            : "={x0}"(o0), "={x1}"(o1), "={x2}"(o2), "={x3}"(o3), "={x4}"(o4), "={x5}"(o5), "={x6}"(o6), "={x7}"(ecode)
            : "i"($sys), "{x0}"(i0), "{x1}"(i1), "{x2}"(i2), "{x3}"(i3), "{x4}"(i4), "{x5}"(i5), "{x6}"(i6)
            : "memory"
            : "volatile" );
        err_or!(ecode, (o0, o1, o2, o3, o4, o5, o6))
    });
}
