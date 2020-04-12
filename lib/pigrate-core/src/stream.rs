use alloc::collections::VecDeque;
use alloc::vec::Vec;

use shim::io;
use crate::message::{Message, Wrapper};

const PACKET_START_MAGIC: [u8; 8] = [0xec, 0xfb, 0x27, 0x3f, 0x06, 0x34, 0x14, 0x8c];
const PACKET_RESYNC_MAGIC: [u8; 8] = [0x7b, 0x64, 0xe1, 0x68, 0x93, 0xd9, 0x0d, 0x01];

// enum State {
//     ExpectingStart,
//     ExpectingLength,
//
// }

pub struct Decoder<R: io::Read, F: FnMut(Message)> {
    pub buffer: VecDeque<u8>,
    pub is_desynced: bool,
    pub reader: R,
    pub handler: F,
}

impl<R: io::Read, F: FnMut(Message)> Decoder<R, F> {

    pub fn new(reader: R, handler: F) -> Self {
        Self { buffer: VecDeque::new(), is_desynced: false, reader, handler }
    }

    fn try_sync(&mut self) -> crate::Result<()> {
        Err(crate::Error::Desync)
    }

    fn buffer_until(&mut self, size: usize) -> crate::Result<()> {
        if size < self.buffer.len() {
            return Ok(())
        }
        let mut left_to_read = size - self.buffer.len();

        let mut buffer = [0u8; 1024];

        while left_to_read > 0 {
            let amt = self.reader.read(&mut buffer)?;
            if amt == 0 {
                return Err(crate::Error::Waiting);
            }

            self.buffer.reserve(amt);
            self.buffer.extend(&buffer[..amt]);
            left_to_read -= core::cmp::min(amt, left_to_read);
        }

        Ok(())
    }

    fn process_packet(&mut self) -> crate::Result<()> {
        self.buffer_until(8)?;

        if self.buffer.iter().take(8).zip(PACKET_START_MAGIC.iter()).any(|(a, b)| *a != *b) {
            return Err(crate::Error::Desync);
        }

        self.buffer_until(16)?;

        let raw_size: Vec<u8> = self.buffer.iter().cloned().skip(8).take(8).collect();
        let mut raw_size_arr: [u8; 8] = [0; 8];
        raw_size_arr.clone_from_slice(raw_size.as_slice());

        // assumes little endian
        let size: u64 = unsafe { core::mem::transmute(raw_size_arr) };

        self.buffer_until(size as usize + 16)?;

        let mut encoded_wrapper: Vec<u8> = Vec::new();
        // we want 1 slice ... also clear the deque for the next packet.
        encoded_wrapper.reserve(size as usize);
        encoded_wrapper.extend(self.buffer.drain(..(size as usize + 16)).skip(16));

        let wrapper: Wrapper = serde_cbor::de::from_slice(encoded_wrapper.as_slice())?;

        let message = Message::from_wrapper(&wrapper)?;

        (self.handler)(message);

        Ok(())
    }

    pub fn do_some_work(&mut self) -> crate::Result<()> {
        if self.is_desynced {
            return self.try_sync();
        }

        loop {
            self.process_packet()?;
        }
    }

}

pub struct Encoder<W: io::Write> {
    pub buffer: VecDeque<u8>,
    pub writer: W,
}

impl<W: io::Write> Encoder<W> {
    pub fn new(writer: W) -> Self {
        Self { buffer: VecDeque::new(), writer }
    }

    pub fn send_some(&mut self) -> crate::Result<()> {
        if self.buffer.len() == 0 {
            return Ok(());
        }

        'outer_loop: while self.buffer.len() > 0 {
            let slices = self.buffer.as_slices();

            for slice in [slices.0, slices.1].iter() {
                let slice = *slice;
                if slice.len() > 0 {
                    let amt = self.writer.write(slices.0)?;
                    if amt == 0 {
                        // we didnt write anything, exit early.
                        return Err(crate::Error::Waiting);
                    }

                    for _ in 0..amt {
                        self.buffer.pop_front();
                    }

                    continue 'outer_loop;
                }
            }
        }

        Ok(())
    }

    pub fn send_message(&mut self, message: &Message) -> crate::Result<()> {
        self.send_some()?;

        let wrapper = message.as_wrapper()?;

        let encoded = serde_cbor::to_vec(&wrapper)?;
        let size = encoded.len() as u64;

        self.buffer.reserve(size as usize + 16);

        let size: [u8; 8] = unsafe { core::mem::transmute(size) };

        self.buffer.extend(PACKET_START_MAGIC.iter());
        self.buffer.extend(size.iter());
        self.buffer.extend(encoded.iter());


        Ok(())
    }
}



