use alloc::borrow::ToOwned;
use alloc::vec::Vec;

use syscall::flag::{O_CLOEXEC, O_RDONLY};
use syscall::{Error, EINTR};

use redox_rt::proc::*;

pub fn main() -> ! {
    let envs = {
        let mut env = [0_u8; 4096];

        let fd = FdGuard::new(
            syscall::open("/scheme/sys/env", O_RDONLY).expect("bootstrap: failed to open env"),
        );
        let bytes_read = syscall::read(*fd, &mut env).expect("bootstrap: failed to read env");

        if bytes_read >= env.len() {
            // TODO: Handle this, we can allocate as much as we want in theory.
            panic!("env is too large");
        }
        let env = &mut env[..bytes_read];

        let raw_iter = || env.split(|c| *c == b'\n').filter(|var| !var.is_empty());

        let iter = || raw_iter().filter(|var| !var.starts_with(b"INITFS_"));

        iter().map(|var| var.to_owned()).collect::<Vec<_>>()
    };

    extern "C" {
        // The linker script will define this as the location of the initfs header.
        static __initfs_header: u8;
    }

    let initfs_length = unsafe {
        (*(core::ptr::addr_of!(__initfs_header) as *const redox_initfs::types::Header)).initfs_size
    };

    unsafe {
        // Creating a reference to NULL is UB. Mask the UB for now using black_box.
        // FIXME use a raw pointer and inline asm for reading instead for the initfs header.
        spawn_initfs(
            core::ptr::addr_of!(__initfs_header),
            initfs_length.get() as usize,
        );
    }
    const CWD: &[u8] = b"/scheme/initfs";
    const DEFAULT_SCHEME: &[u8] = b"initfs";
    let extrainfo = ExtraInfo {
        cwd: Some(CWD),
        default_scheme: Some(DEFAULT_SCHEME),
        sigprocmask: 0,
        sigignmask: 0,
        umask: redox_rt::sys::get_umask(),
    };

    let path = "/scheme/initfs/bin/init";
    let total_args_envs_auxvpointee_size = path.len()
        + 1
        + envs.len()
        + envs.iter().map(|v| v.len()).sum::<usize>()
        + CWD.len()
        + 1
        + DEFAULT_SCHEME.len()
        + 1;

    let image_file = FdGuard::new(syscall::open(path, O_RDONLY).expect("failed to open init"));
    let open_via_dup = FdGuard::new(
        syscall::open("/scheme/thisproc/current/open_via_dup", 0)
            .expect("failed to open open_via_dup"),
    );
    let memory = FdGuard::new(syscall::open("/scheme/memory", 0).expect("failed to open memory"));

    fexec_impl(
        image_file,
        open_via_dup,
        &memory,
        path.as_bytes(),
        [path],
        envs.iter(),
        total_args_envs_auxvpointee_size,
        &extrainfo,
        None,
    )
    .expect("failed to execute init");

    unreachable!()
}

unsafe fn spawn_initfs(initfs_start: *const u8, initfs_length: usize) {
    let read = syscall::open("/scheme/pipe", O_CLOEXEC).expect("failed to open sync read pipe");

    // The write pipe will not inherit O_CLOEXEC, but is closed by the daemon later.
    let write = syscall::dup(read, b"write").expect("failed to open sync write pipe");

    match fork_impl() {
        Err(err) => {
            panic!("Failed to fork in order to start initfs daemon: {}", err);
        }
        // Continue serving the scheme as the child.
        Ok(0) => {
            let _ = syscall::close(read);
        }
        // Return in order to execute init, as the parent.
        Ok(_) => {
            let _ = syscall::close(write);
            loop {
                match syscall::read(read, &mut [0]) {
                    Err(Error { errno: EINTR }) => continue,
                    _ => break,
                }
            }

            return;
        }
    }
    crate::initfs::run(
        core::slice::from_raw_parts(initfs_start, initfs_length),
        write,
    );
}
