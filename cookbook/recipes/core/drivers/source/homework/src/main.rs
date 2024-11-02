extern crate syscall;

use redox_scheme::SchemeBlockMut;
use std::io;
use std::io::{Read, Write};
use syscall::data::Packet;

use std::fs::File;

mod scheme;

use scheme::SlotScheme;

// The runtime of the driver
fn main() -> io::Result<()> {
    println!("Started the `homework` driver");

    redox_daemon::Daemon::new(move |daemon| {
        let mut slot_scheme = SlotScheme::new();

        let mut socket = File::create(":homework").expect("homeworkd: failed to create homework scheme");
        daemon.ready().expect("failed to notify parent");

        loop {
            let mut packet = Packet::default();
            socket
                .read(&mut packet)
                .expect("homeworkd: failed to read events from homework scheme");
            //println!("Initial packet: {:?}", packet);
            if let Some(response) = unsafe { slot_scheme.handle(&mut packet) } {
                packet.a = response;
            //println!("Modified packet: {:?}", packet);
            socket
                .write(&packet)
                .expect("homeworkd: failed to write responses to homework scheme");
            } else {
                panic!("Something's gone very wrong!");
            }
        }
    })
    .expect("homeworkd: failed to daemonize");

}
