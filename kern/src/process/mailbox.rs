use crate::process::Id;
use alloc::collections::VecDeque;

type ReplyTo = Id;

pub enum Message {
    ListenStateDead(ReplyTo),

    NotifyStateDead(Id),
}

pub struct Mailbox {
    pub queue: VecDeque<Message>,
}

impl Mailbox {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}


