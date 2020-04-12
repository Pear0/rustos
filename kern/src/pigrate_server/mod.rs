use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::time::Duration;

use shim::{io, ioerr};

use crate::{NET, SCHEDULER};
use crate::iosync::{SyncRead, SyncWrite};
use crate::net::ipv4;
use crate::process::{Id, Process};
use crate::process::fd::FileDescriptor;
use crate::sync::Waitable;

pub fn pigrate_server() -> ! {
    let pid: Id = kernel_api::syscall::getpid();
    let (source, sink) = SCHEDULER.crit_process(pid, |f| {
        let f = f.unwrap();
        (f.file_descriptors[0].read.as_ref().unwrap().clone(), f.file_descriptors[1].write.as_ref().unwrap().clone())
    });

    loop {
        let mut buf_array = [0u8; 1024];
        let mut buf = &mut buf_array[..];

        match source.read(&mut buf) {
            Ok(n) if n > 0 => buf = &mut buf[..n],
            _ => {
                kernel_api::syscall::sleep(Duration::from_millis(1));
                continue;
            }
        }

        'write_loop: while buf.len() > 0 {
            match sink.write(&buf) {
                Ok(n) if n > 0 => buf = &mut buf[n..],
                _ => {
                    kernel_api::syscall::sleep(Duration::from_millis(1));
                    continue 'write_loop;
                }
            }
        }
    }
}

pub fn register_pigrate() {
    NET.critical(|net| {
        let my_ip = net.ip.address();

        net.tcp.add_listening_port((my_ip, 200), Box::new(|sink, source| {
            let mut proc = Process::kernel_process_old(String::from("pigrate server"), pigrate_server)
                .or(ioerr!(Other, "foo"))?;

            proc.file_descriptors.push(FileDescriptor::read(Arc::new(source)));
            proc.file_descriptors.push(FileDescriptor::write(Arc::new(sink)));

            SCHEDULER.add(proc);

            Ok(())
        }));
    });
}



