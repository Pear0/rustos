use alloc::vec::Vec;
use crate::bundle::ProcessBundle;

use crate::Error;
use serde::Deserialize;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Wrapper {
    pub tag: i32,
    pub blob: Vec<u8>,
}

impl Wrapper {
    pub fn new(tag: i32, blob: Vec<u8>) -> Self {
        Wrapper { tag, blob }
    }
}

#[derive(Debug)]
pub enum Message {
    StartBundle(StartBundle),
    Echo(Echo),
}

const TAG_START_BUNDLE: i32 = 1;
const TAG_ECHO: i32 = 2;

impl Message {

    fn do_decode<'a, T: Deserialize<'a>>(blob: &'a Vec<u8>) -> crate::Result<T> {
        Ok(serde_cbor::from_slice(blob.as_slice())?)
    }

    pub fn from(tag: i32, blob: &Vec<u8>) -> crate::Result<Self> {
        Ok(match tag {
            TAG_START_BUNDLE => Message::StartBundle(Self::do_decode(blob)?),
            TAG_ECHO => Message::Echo(Self::do_decode(blob)?),
            _ => return Err(Error::UnknownTag(tag)),
        })
    }

    pub fn from_wrapper(wrapper: &Wrapper) -> crate::Result<Self> {
        Self::from(wrapper.tag, &wrapper.blob)
    }

    pub fn as_wrapper(&self) -> crate::Result<Wrapper> {
        Ok(match self {
            Message::StartBundle(b) => Wrapper::new(TAG_START_BUNDLE, serde_cbor::to_vec(b)?),
            Message::Echo(e) => Wrapper::new(TAG_ECHO, serde_cbor::to_vec(e)?),
        })
    }

}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Echo {
    pub data: Vec<u8>,
}

impl From<&str> for Echo {
    fn from(s: &str) -> Self {
        let mut v: Vec<u8> = Vec::new();
        v.extend(s.as_bytes());
        Self { data: v }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StartBundle {
    pub bundle: ProcessBundle,
}
