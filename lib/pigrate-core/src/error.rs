use shim::io;

#[derive(Debug)]
pub enum Error {
    Serde(serde_cbor::Error),
    UnknownTag(i32),
    Io(io::Error),
    Waiting,
    Desync,
}

impl From<serde_cbor::Error> for Error {
    fn from(e: serde_cbor::Error) -> Self {
        Error::Serde(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}


