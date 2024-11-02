use redox_scheme::SchemeBlockMut;
use std::collections::BTreeMap;
use std::convert::TryInto;
use syscall::error::{Error, Result, EBADF, EFAULT, EINVAL};

// We don't need this handle, but it's a good illustration of how one would work
struct Handle {
    // The slot to which this handle is pointing
    current_slot: usize,
}

pub struct SlotScheme {
    next_id: usize,
    slots: [usize; 4],
    current_slot: usize,
    handles: BTreeMap<usize, Handle>,
}

impl SlotScheme {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            slots: [0; 4],
            current_slot: 0,
            handles: BTreeMap::new(),
        }
    }
}

// FOR DEBUGGING
// I left this in because it became handy
use core::{mem, slice, str};

use redox_scheme::*;
use syscall::data::*;
use syscall::error::*;
use syscall::flag::*;
use syscall::number::*;

pub(crate) fn convert_in_scheme_handle_block(
    _: &Packet,
    result: Result<Option<OpenResult>>,
) -> Result<Option<usize>> {
    match result {
        Ok(Some(OpenResult::ThisScheme { number, .. })) => Ok(Some(number)),
        Ok(Some(OpenResult::OtherScheme { .. })) => Err(Error::new(EOPNOTSUPP)),
        Ok(None) => Ok(None),
        Err(err) => Err(err),
    }
}

unsafe fn str_from_raw_parts(ptr: *const u8, len: usize) -> Option<&'static str> {
    let slice = slice::from_raw_parts(ptr, len);
    str::from_utf8(slice).ok()
}

// Implementing `SchemeBlockMut` provides the `handle` function used in `main`
impl SchemeBlockMut for SlotScheme {
    // `handle` has a default implementation, so we normally don't need to deal with all this
    // I left this in because it became handy
    unsafe fn handle(&mut self, packet: &Packet) -> Option<usize> {
        let res = match packet.a {
            SYS_OPEN => {
                println!("SYS_OPEN");
                if let Some(path) = str_from_raw_parts(packet.b as *const u8, packet.c) {
                    convert_in_scheme_handle_block(
                        packet,
                        self.xopen(path, packet.d, &CallerCtx::from_packet(&packet)),
                    )
                } else {
                    Err(Error::new(EINVAL))
                }
            }
            SYS_RMDIR => {
                if let Some(path) = str_from_raw_parts(packet.b as *const u8, packet.c) {
                    self.rmdir(path, packet.uid, packet.gid)
                } else {
                    Err(Error::new(EINVAL))
                }
            }
            SYS_UNLINK => {
                if let Some(path) = str_from_raw_parts(packet.b as *const u8, packet.c) {
                    self.unlink(path, packet.uid, packet.gid)
                } else {
                    Err(Error::new(EINVAL))
                }
            }

            SYS_DUP => convert_in_scheme_handle_block(
                packet,
                self.xdup(
                    packet.b,
                    slice::from_raw_parts(packet.c as *const u8, packet.d),
                    &CallerCtx::from_packet(&packet),
                ),
            ),
            SYS_READ => {
                println!("SYS_READ");
                let buf = slice::from_raw_parts_mut(packet.c as *mut u8, packet.d);
                //println!("buf: {:?}", buf);
                self.read_old(packet.b, buf)
            }
            SYS_WRITE => {
                println!("SYS_WRITE");
                let buf = slice::from_raw_parts_mut(packet.c as *mut u8, packet.d);
                //println!("buf: {:?}", buf);
                self.write_old(packet.b, buf)
            }
            SYS_FCHMOD => self.fchmod(packet.b, packet.c as u16),
            SYS_FCHOWN => self.fchown(packet.b, packet.c as u32, packet.d as u32),
            SYS_FCNTL => self.fcntl(packet.b, packet.c, packet.d),
            SYS_FEVENT => self
                .fevent(packet.b, EventFlags::from_bits_truncate(packet.c))
                .map(|f| f.map(|f| f.bits())),
            SYS_FPATH => self.fpath(packet.b, {
                slice::from_raw_parts_mut(packet.c as *mut u8, packet.d)
            }),
            SYS_FRENAME => {
                if let Some(path) = str_from_raw_parts(packet.c as *const u8, packet.d) {
                    self.frename(packet.b, path, packet.uid, packet.gid)
                } else {
                    Err(Error::new(EINVAL))
                }
            }
            SYS_FSTAT => {
                if packet.d >= mem::size_of::<Stat>() {
                    self.fstat(packet.b, &mut *(packet.c as *mut Stat))
                } else {
                    Err(Error::new(EFAULT))
                }
            }
            SYS_FSTATVFS => {
                if packet.d >= mem::size_of::<StatVfs>() {
                    self.fstatvfs(packet.b, &mut *(packet.c as *mut StatVfs))
                } else {
                    Err(Error::new(EFAULT))
                }
            }
            SYS_FSYNC => self.fsync(packet.b),
            SYS_FTRUNCATE => {
                println!("SYS_FTRUNCATE");
                self.ftruncate(packet.b, packet.c)
            }
            SYS_FUTIMENS => {
                if packet.d >= mem::size_of::<TimeSpec>() {
                    self.futimens(packet.b, {
                        slice::from_raw_parts(
                            packet.c as *const TimeSpec,
                            packet.d / mem::size_of::<TimeSpec>(),
                        )
                    })
                } else {
                    Err(Error::new(EFAULT))
                }
            }
            SYS_CLOSE => {
                println!("SYS_CLOSE");
                self.close(packet.b)}

            KSMSG_MMAP_PREP => self.mmap_prep(
                packet.b,
                u64::from(packet.uid) | (u64::from(packet.gid) << 32),
                packet.c,
                MapFlags::from_bits_truncate(packet.d),
            ),
            KSMSG_MUNMAP => self.munmap(
                packet.b,
                u64::from(packet.uid) | (u64::from(packet.gid) << 32),
                packet.c,
                MunmapFlags::from_bits_truncate(packet.d),
            ),

            _ => Err(Error::new(ENOSYS)),
        };

        res.transpose().map(Error::mux)
    }

    fn open(&mut self, _path: &str, _flags: usize, _uid: u32, _gid: u32) -> Result<Option<usize>> {
        // `open` increments to the next id
        println!("driver: open()");
        self.next_id += 1;
        let id = self.next_id;
        self.handles.insert(id, Handle { current_slot: 0 });
        println!("driver: open() created handle {id}");
        println!("driver: current slots: {:?}", self.slots);
        Ok(Some(id))
    }

    fn read_old(
        &mut self,
        id: usize,
        buf: &mut [u8], /*, _offset: u64, _flags: u32*/
    ) -> Result<Option<usize>> {
        // `read` will succeed if the id exists
        println!("driver: read() id {id}");
        //let handle = self.handles.get_mut(&id).ok_or(Error::new(EBADF))?;

        println!("current slot: {:?}", self.current_slot);

        // If the slot number is valid, print the value
        let slot_value = self
            .slots
            .get(self.current_slot)
            .ok_or(Error::new(EFAULT))? // 'Bad address'
            .to_ne_bytes();
        buf[0] = slot_value[0];
        buf[1] = slot_value[1];
        buf[2] = slot_value[2];
        buf[3] = slot_value[3];
        //println!("Slot {} current value: {}", self.current_slot, slot_value);
        Ok(Some(0))
    }

    fn write_old(
        &mut self,
        id: usize,
        buf: &[u8], /*, _offset: u64, _flags: u32*/
    ) -> Result<Option<usize>> {
        println!("driver: write() id {id}");
        //let handle = self.handles.get_mut(&id).ok_or(Error::new(EBADF))?;
        println!("driver: write() current slot: {:?}", self.current_slot);

        // Parse input
        let (int_bytes, rest) = buf.split_at(std::mem::size_of::<usize>());
        if rest.len() > 0 {
            // The value input wasn't an integer
            return Err(Error::new(EINVAL)); // Invalid arg
        }
        let value = usize::from_ne_bytes(int_bytes.try_into().map_err(|_| Error::new(EINVAL))?);
        println!("driver: write() value {value}");
        // Write the new value to the slot
        *self
            .slots
            .get_mut(self.current_slot)
            .ok_or(Error::new(EFAULT))? = value;
        Ok(Some(0))
    }

    // hijack this method
    fn ftruncate(&mut self, _id: usize, len: usize) -> Result<Option<usize>> {
        if len > 3 {
            Err(Error::new(EINVAL))
        } else {
            self.current_slot = len;
            Ok(Some(0))
        }
    }

    fn close(&mut self, id: usize) -> Result<Option<usize>> {
        println!("driver: close() id {id}");
        // `close` removes an open id if it exists
        println!("handles: {:?}", self.handles.keys());
        let _handle = self.handles.remove(&id).ok_or(Error::new(EBADF))?;
        Ok(Some(0))
    }
}
