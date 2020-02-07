mod parsers;

use serial;
use structopt;
use structopt_derive::StructOpt;
use xmodem::{Xmodem, Progress};

use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use structopt::StructOpt;
use serial::core::{CharSize, BaudRate, StopBits, FlowControl, SerialDevice, SerialPortSettings};

use parsers::{parse_width, parse_stop_bits, parse_flow_control, parse_baud_rate};
use serial::{SerialPort, PortSettings, SystemPort};

#[derive(StructOpt, Debug)]
#[structopt(about = "Write to TTY using the XMODEM protocol by default.")]
struct Opt {
    #[structopt(short = "i", help = "Input file (defaults to stdin if not set)", parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(short = "b", long = "baud", parse(try_from_str = "parse_baud_rate"),
    help = "Set baud rate", default_value = "115200")]
    baud_rate: BaudRate,

    #[structopt(short = "t", long = "timeout", parse(try_from_str),
    help = "Set timeout in seconds", default_value = "10")]
    timeout: u64,

    #[structopt(short = "w", long = "width", parse(try_from_str = "parse_width"),
    help = "Set data character width in bits", default_value = "8")]
    char_width: CharSize,

    #[structopt(help = "Path to TTY device", parse(from_os_str))]
    tty_path: PathBuf,

    #[structopt(short = "f", long = "flow-control", parse(try_from_str = "parse_flow_control"),
    help = "Enable flow control ('hardware' or 'software')", default_value = "none")]
    flow_control: FlowControl,

    #[structopt(short = "s", long = "stop-bits", parse(try_from_str = "parse_stop_bits"),
    help = "Set number of stop bits", default_value = "1")]
    stop_bits: StopBits,

    #[structopt(short = "r", long = "raw", help = "Disable XMODEM")]
    raw: bool,
}

fn progress_fn(progress: xmodem::Progress) {
    match progress {
        Progress::Waiting => {}
        Progress::Started => {}
        Progress::Packet(_) => print!("."),
        _ => println!("Progress: {:?}", progress),
    }
    io::stdout().flush().unwrap();
}


fn send_full(input: &mut dyn io::Read, port: &mut SystemPort, raw: bool) -> io::Result<usize> {
    if raw {
        return match io::copy(input, port) {
            Ok(t) => Ok(t as usize),
            Err(e) => Err(e),
        };
    }

    Xmodem::transmit_with_progress(input, port, progress_fn)
}


fn main() {
    use std::fs::File;
    use std::io::{self, BufReader};

    let opt = Opt::from_args();
    let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");

    port.configure(&PortSettings {
        baud_rate: opt.baud_rate,
        char_size: opt.char_width,
        parity: serial::ParityNone,
        stop_bits: opt.stop_bits,
        flow_control: opt.flow_control,
    }).unwrap();

    SerialDevice::set_timeout(&mut port, Duration::from_secs(opt.timeout)).unwrap();

    for i in 0..1 {
        println!("Sending...");

        'retry: loop {
            let mut source: Box<dyn io::Read>;
            let try_num;
            if let Some(path) = &opt.input {
                source = Box::new(File::open(path).expect("failed to open file"));
                try_num = 1;
            } else {
                source = Box::new(io::stdin());
                try_num = 1;
            }

            match send_full(source.as_mut(), &mut port, opt.raw) {
                Ok(_) => {
                    println!("\nSuccess");
                    return;
                }
                Err(ref e) if e.kind() == io::ErrorKind::InvalidData => continue 'retry,
                Err(e) => {
                    println!("\nFailed to send: {}", e);
                    break 'retry;
                }
            }
        }
    }
}
