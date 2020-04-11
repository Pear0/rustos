use alloc::sync::Arc;
use crate::fs::handle::{Source, Sink};

#[derive(Clone)]
pub struct FileDescriptor {
    pub read: Option<Arc<Source>>,
    pub write: Option<Arc<Sink>>,
}

impl FileDescriptor {
    pub fn read(source: Arc<Source>) -> Self {
        Self { read: Some(source), write: None }
    }

    pub fn write(sink: Arc<Sink>) -> Self {
        Self { read: None, write: Some(sink) }
    }

    pub fn read_write(source: Arc<Source>, sink: Arc<Sink>) -> Self {
        Self { read: Some(source), write: Some(sink) }
    }

}
