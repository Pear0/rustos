use alloc::sync::Arc;

use hashbrown::HashMap;

use crate::fs::handle::{Sink, Source};
use crate::iosync::Global;
use crate::kernel_call::syscall::wait_waitable;
use crate::sync::Waitable;
use crate::iosync::{SyncRead, SyncWrite};

static PIPE_SERVICE: Global<PipeService> = Global::new(|| PipeService::new());

pub struct PipeService {
    id: usize,
    pipes: HashMap<usize, (Sink, Source)>,
}

impl PipeService {
    pub fn new() -> Self {
        Self {
            id: 1,
            pipes: HashMap::new(),
        }
    }

    pub fn register(sink: Sink, source: Source) -> usize {
        PIPE_SERVICE.critical(|pipe| {
            let id = pipe.id;
            pipe.id += 1;
            pipe.pipes.insert(id, (sink, source));
            id
        })
    }

    pub fn unregister(id: usize) {
        PIPE_SERVICE.critical(|pipe| {
            pipe.pipes.remove(&id);
        })
    }

    pub fn task_func() -> ! {
        let waitable = Arc::new(PipeWaitable());

        loop {
            PIPE_SERVICE.critical(|pipe| {
                'copies: for (sink, source) in pipe.pipes.values() {
                    if sink.done_waiting() && source.done_waiting() {
                        let mut buf = [0u8; 64];

                        // TODO don't ignore errors

                        loop {
                            let read = match source.read(&mut buf) {
                                Ok(0) => continue 'copies,
                                Err(_) => continue 'copies,
                                Ok(n) => n,
                            };

                            match sink.write(&buf[..read]) {
                                Err(_) => continue 'copies,
                                Ok(written) => {
                                    if written != read {
                                        // TODO handle an incomplete write.
                                        continue 'copies;
                                    }
                                }
                            }
                        }

                    }
                }
            });

            wait_waitable(waitable.clone());
        }
    }
}

struct PipeWaitable();

impl Waitable for PipeWaitable {
    fn done_waiting(&self) -> bool {
        PIPE_SERVICE.critical(|pipe| {

            // is there work we can do
            for (sink, source) in pipe.pipes.values() {
                if sink.done_waiting() && source.done_waiting() {
                    return true;
                }
            }

            false
        })
    }
}



