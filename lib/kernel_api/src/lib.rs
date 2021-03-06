#![feature(llvm_asm)]
#![allow(unused_parens)]
#![no_std]

use core::fmt;

use shim::io;

#[cfg(feature = "user-space")]
pub mod syscall;

pub mod hypercall;

#[macro_use]
mod hypercall_macros;
#[macro_use]
mod syscall_macros;

pub type OsResult<T> = core::result::Result<T, OsError>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OsError {
    Unknown = 0,
    Ok = 1,

    NoEntry = 10,
    NoMemory = 20,
    NoVmSpace = 30,
    NoAccess = 40,
    BadAddress = 50,
    FileExists = 60,
    InvalidArgument = 70,

    IoError = 101,
    IoErrorEof = 102,
    IoErrorInvalidData = 103,
    IoErrorInvalidInput = 104,
    IoErrorTimedOut = 105,

    InvalidSocket = 200,
    SocketAlreadyOpen = 201,
    InvalidPort = 202,

    // custom
    Waiting = 500,
}

impl core::convert::From<u64> for OsError {
    fn from(e: u64) -> Self {
        match e {
            1 => OsError::Ok,

            10 => OsError::NoEntry,
            20 => OsError::NoMemory,
            30 => OsError::NoVmSpace,
            40 => OsError::NoAccess,
            50 => OsError::BadAddress,
            60 => OsError::FileExists,
            70 => OsError::InvalidArgument,

            101 => OsError::IoError,
            102 => OsError::IoErrorEof,
            103 => OsError::IoErrorInvalidData,
            104 => OsError::IoErrorInvalidInput,

            200 => OsError::InvalidSocket,
            201 => OsError::SocketAlreadyOpen,
            202 => OsError::InvalidPort,

            500 => OsError::Waiting,

            _ => OsError::Unknown,
        }
    }
}

impl core::convert::From<io::Error> for OsError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::UnexpectedEof => OsError::IoErrorEof,
            io::ErrorKind::InvalidData => OsError::IoErrorInvalidData,
            io::ErrorKind::InvalidInput => OsError::IoErrorInvalidInput,
            io::ErrorKind::TimedOut => OsError::IoErrorTimedOut,
            io::ErrorKind::NotFound => OsError::NoEntry,
            _ => OsError::IoError,
        }
    }
}

#[macro_export]
macro_rules! err_or {
    ($ecode:expr, $rtn:expr) => {{
        let e = OsError::from($ecode);
        if let OsError::Ok = e {
            Ok($rtn)
        } else {
            Err(e)
        }
    }};
}

/************/
/* syscalls */
/************/

pub const NR_SLEEP: usize = 1;
pub const NR_TIME: usize = 2;
pub const NR_EXIT: usize = 3;
pub const NR_WRITE: usize = 4;
pub const NR_GETPID: usize = 5;
pub const NR_WAITPID: usize = 6;
pub const NR_SBRK: usize = 7;

/**************/
/* hypercalls */
/**************/

// Virtualized NIC hypercalls
pub const HP_VNIC_STATE: usize = 1;
pub const HP_VNIC_GET_INFO: usize = 2;
pub const HP_VNIC_SEND: usize = 3;
pub const HP_VNIC_RECEIVE: usize = 4;






