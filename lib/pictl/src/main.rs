
extern crate pigrate_core;

#[macro_use]
extern crate serde;

use std::env;
use std::fs::File;
use std::{io, mem};
use pigrate_core::FakeTrapFrame;
use pigrate_core::bundle::{MemoryBundle, ProcessBundle};
use std::net::TcpStream;
use std::time::Duration;
use pigrate_core::message::{Message, Echo};
use pigrate_core::Error;

pub const PAGE_SIZE: usize = 64 * 1024;
pub const USER_IMG_BASE: usize = 0xffff_ffff_c000_0000;
pub const STACK_BASE: usize = 0xffffffffffff0000;

fn run_program(args: &[String]) {
    if args.len() == 0 {
        println!("usage: run <file>");
        return;
    }

    let mut file = File::open(&args[0]).expect("could not open file");

    let mut binary: Vec<u8> = Vec::new();

    io::copy(&mut file, &mut binary).expect("failed to read all of file");
    mem::drop(file);

    let mut frame = FakeTrapFrame::default();
    frame.elr = USER_IMG_BASE as u64;
    frame.sp = STACK_BASE.wrapping_add(PAGE_SIZE) as u64;
    frame.spsr = 0x40000000;

    let mut memory = MemoryBundle::default();

    {
        let mut page: Vec<u8> = Vec::new();
        page.resize(PAGE_SIZE, 0);
        memory.generic_pages.insert(STACK_BASE as u64, page);
    }

    for (i, chunk) in binary.chunks(PAGE_SIZE).enumerate() {
        let mut page: Vec<u8> = Vec::new();
        page.extend(chunk.iter());
        page.resize(PAGE_SIZE, 0);
        memory.generic_pages.insert((USER_IMG_BASE + i * PAGE_SIZE) as u64, page);
    }


    let mut frame_enc = Vec::<u8>::new();
    frame_enc.extend_from_slice(frame.as_bytes());

    let proc = ProcessBundle { frame: frame_enc, name: String::from("fib2.bin"), memory };

    let encoded = serde_cbor::ser::to_vec(&proc).expect("failed to serialize");

    use compression::prelude::*;

    let compressed = encoded.iter().cloned().encode(&mut GZipEncoder::new(), Action::Finish).collect::<Result<Vec<_>, _>>().expect("failed to compress");

    println!("{}", hex::encode(compressed.as_slice()));
}

fn ping_program(args: &[String]) -> io::Result<()> {
    use std::io::*;

    let mut stream = TcpStream::connect("169.254.78.130:200")?;

    {
        let mut encoder = pigrate_core::stream::Encoder::new(&mut stream);

        for i in 0..100 {
            encoder.send_message(&Message::Echo(Echo::from("Hello world")));

            loop {
                match encoder.send_some() {
                    Ok(_) => break,
                    Err(pigrate_core::Error::Waiting) => {},
                    Err(e) => println!("{:?}", e),
                }
            }
        }
    }

    {
        let mut decoder = pigrate_core::stream::Decoder::new(&mut stream, |message| {
            println!("Message: {:?}", message);
        });

        loop {
            decoder.do_some_work();
        }
    }

    //
    // stream.write("Hello world\n".as_bytes())?;
    //
    // stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    //
    // io::copy(&mut stream, &mut stdout())?;

    Ok(())
}


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("requires a command: run, ping");
        return;
    }

    match args[1].as_str() {
        "run" => run_program(&args[2..]),
        "ping" => ping_program(&args[2..]).unwrap(),
        _ => {
            println!("unknown command: {}", args[1]);
        }
    }
}
