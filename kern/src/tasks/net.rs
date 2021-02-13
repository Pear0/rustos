use core::time::Duration;
use kernel_api::syscall::sleep;
use crate::NET;
use crate::net::ipv4;
use crate::process::KernProcessCtx;

pub fn testing_send_thread(ctx: KernProcessCtx) {
    let dest: ipv4::Address = "239.15.55.200".parse().unwrap();

    loop {
        if !NET.is_initialized() {
            sleep(Duration::from_secs(2));
            continue;
        }

        if let Err(e) = NET.critical(|net| {
            let msg = "hello world!";
            net.send_datagram(dest, 4001, 4000, msg.as_bytes())
        }) {
            info!("error sending: {:?}", e);
        }

        sleep(Duration::from_secs(1));
    }
}



