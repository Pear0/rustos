mod parsers;

use serial;
use structopt;
use structopt_derive::StructOpt;
use xmodem::Xmodem;

use std::io;
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
    println!("Progress: {:?}", progress);
}


fn send_full(input: &mut dyn io::Read, port: &mut SystemPort, raw: bool) {
    if raw {
        io::copy(input, port).expect("failed to write raw");
        return;
    }

    Xmodem::transmit_with_progress(input, port, progress_fn).expect("failed to write Xmodem");
}


fn main() {
    use std::fs::File;
    use std::io::{self, BufReader};

    let opt = Opt::from_args();
    let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");

    port.configure(&PortSettings{
        baud_rate: opt.baud_rate,
        char_size: opt.char_width,
        parity: serial::ParityNone,
        stop_bits: opt.stop_bits,
        flow_control: opt.flow_control,
    });

    SerialDevice::set_timeout(&mut port, Duration::from_secs(opt.timeout));

    match opt.input {
        Some(path) => {
            let file = File::open(path).expect("failed to open file");
            let mut buf_reader = BufReader::new(file);

            send_full(&mut buf_reader, &mut port, opt.raw);
        }
        None => {
            send_full(&mut io::stdin(), &mut port, opt.raw);
        }
    }

}
