use core::fmt;
use core::ops::Generator;

use crate::result::{Error, Result, UsizeError};
use crate::syscalls::{allow, command, subscribe, CallbackMessage};
use crate::task::{DriverTask, DriverTaskClient, DriverTaskWithState};

const DRIVER_NUM: usize = 1;

mod allow_num {
    pub const WRITE: usize = 1;
}

mod subscribe_num {
    pub const WRITE: usize = 1;
}

mod command_num {
    pub const WRITE: usize = 1;
}

static mut CONSOLE_WRITE_MESSAGE: Option<CallbackMessage> = None;

pub struct BytesWritten(usize);

#[derive(Copy, Clone)]
pub enum ConsoleWriteClientMessage {
    BytesWritten(Result<usize>),
}

static mut CONSOLE_WRITE_CLIENT_MESSAGE: Option<ConsoleWriteClientMessage> = None;

extern "C" fn console_write_callback(arg0: usize, arg1: usize, arg2: usize, userdata: usize) {
    let cb_message = CallbackMessage::new(arg0, arg1, arg2, userdata);

    unsafe {
        CONSOLE_WRITE_MESSAGE = Some(cb_message);
    }
}

#[derive(Copy, Clone)]
pub struct WritesPending(usize);

#[derive(Copy, Clone)]
pub struct WritesComplete(usize);

// Indicates if there is an ongoing write. Once the write is complete,
// `CONSOLE_WRITE_STATE` is set to None and a client message is sent.
#[derive(Copy, Clone)]
pub enum ConsoleWriteState {
    Ongoing(WritesPending, WritesComplete),
}

static mut CONSOLE_WRITE_STATE: Option<ConsoleWriteState> = None;

// Corresponds to kernel write buffer
static mut CONSOLE_WRITE_BUF: [u8; 64] = [0; 64];

pub struct ConsoleWrite;

impl ConsoleWrite {
    pub fn new() -> ConsoleWrite {
        ConsoleWrite
    }

    // Safety : This coroutine is called whenever there is an incoming callback
    //          message. When called, it *must* consume the incoming callback
    //          message before yielding.
    pub unsafe fn get_task(&self) -> impl Generator<Yield = (), Return = ()> + '_ {
        || loop {
            if let Some(cb_message) = CONSOLE_WRITE_MESSAGE.take() {
                let c = CONSOLE_WRITE_STATE.clone();

                if let Some(ConsoleWriteState::Ongoing(wp, wc)) = c {
                    let x: UsizeError = cb_message.get_arg0().into();
                    match x.0 {
                        Some(e) => {
                            // Callback error
                            CONSOLE_WRITE_STATE = None;
                            CONSOLE_WRITE_CLIENT_MESSAGE =
                                Some(ConsoleWriteClientMessage::BytesWritten(Err(e)));
                        }
                        None => {
                            // No callback error
                            let mut wp = wp.0;
                            let mut wc = wc.0;

                            wc += cb_message.get_arg0();
                            wp -= cb_message.get_arg0();

                            if wp == 0 {
                                CONSOLE_WRITE_STATE = None;
                                CONSOLE_WRITE_CLIENT_MESSAGE =
                                    Some(ConsoleWriteClientMessage::BytesWritten(Ok(wc)));
                            } else {
                                CONSOLE_WRITE_STATE = Some(ConsoleWriteState::Ongoing(
                                    WritesPending(wp),
                                    WritesComplete(wc),
                                ));
                            }
                        }
                    };
                }
            }

            yield;
        }
    }

    pub fn initiate_write(&self, s: &[u8]) -> Result<()> {
        unsafe {
            // is there an ongoing write
            if CONSOLE_WRITE_STATE.is_some() {
                return Err(Error::EBUSY);
            }

            // previous console write client message has not been consumed
            if ConsoleWriteClient::new().has_message() {
                return Err(Error::EBUSY);
            }

            if s.len() > CONSOLE_WRITE_BUF.len() {
                return Err(Error::EINVAL);
            }

            self.clear_console_write_buf();

            &CONSOLE_WRITE_BUF[..s.len()].copy_from_slice(s);

            let res = allow(
                DRIVER_NUM,
                allow_num::WRITE,
                &CONSOLE_WRITE_BUF as *const u8 as *mut u8,
                s.len(),
            )
            .and_then(|_| {
                subscribe(
                    DRIVER_NUM,
                    subscribe_num::WRITE,
                    console_write_callback as *const _,
                    0,
                )
            })
            .and_then(|_| command(DRIVER_NUM, command_num::WRITE, s.len(), 0))
            .map(|_| ())
            .map_err(|e| {
                self.clear_console_write_buf();
                e
            });

            CONSOLE_WRITE_STATE = Some(ConsoleWriteState::Ongoing(
                WritesPending(s.len()),
                WritesComplete(0),
            ));

            res
        }
    }

    fn clear_console_write_buf(&self) {
        unsafe {
            &CONSOLE_WRITE_BUF.iter_mut().for_each(|x| *x = 0);
        }
    }
}

impl DriverTask for ConsoleWrite {
    fn has_message(&self) -> bool {
        unsafe { CONSOLE_WRITE_MESSAGE.is_some() }
    }
}

impl DriverTaskWithState for ConsoleWrite {
    fn is_active(&self) -> bool {
        unsafe { CONSOLE_WRITE_STATE.is_some() }
    }
}

pub struct ConsoleWriteClient;

impl ConsoleWriteClient {
    pub fn new() -> ConsoleWriteClient {
        ConsoleWriteClient
    }

    pub fn reap_bytes_written_message(&self) -> Result<BytesWritten> {
        unsafe {
            let c = CONSOLE_WRITE_CLIENT_MESSAGE.clone();
            let res = c.ok_or(Error::EINVAL).and_then(|c| match c {
                ConsoleWriteClientMessage::BytesWritten(len) => {
                    len.and_then(|l| Ok(BytesWritten(l)))
                }
            });

            CONSOLE_WRITE_CLIENT_MESSAGE = None;

            res
        }
    }
}

impl DriverTaskClient for ConsoleWriteClient {
    fn has_message(&self) -> bool {
        unsafe { CONSOLE_WRITE_CLIENT_MESSAGE.is_some() }
    }

    fn reap_message(&self) {
        unsafe {
            let c = CONSOLE_WRITE_CLIENT_MESSAGE.clone();
            c.map(|_| {
                CONSOLE_WRITE_CLIENT_MESSAGE = None;
            });
        }
    }
}

pub struct ConsoleWriteStr<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> ConsoleWriteStr<'a> {
    pub fn new(buf: &'a mut [u8]) -> ConsoleWriteStr<'a> {
        ConsoleWriteStr {
            buf: buf,
            offset: 0,
        }
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }
}

// https://stackoverflow.com/a/39491059
impl<'a> fmt::Write for ConsoleWriteStr<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // &[u8]
        let bytes = s.as_bytes();

        // Skip over already-copied data
        let remainder = &mut self.buf[self.offset..];
        // Check if there is space remaining (return error instead of panicking)
        if remainder.len() < bytes.len() {
            return Err(core::fmt::Error);
        }
        // Make the two slices the same length
        let remainder = &mut remainder[..bytes.len()];
        // Copy
        remainder.copy_from_slice(bytes);

        // Update offset to avoid overwriting
        self.offset += bytes.len();

        Ok(())
    }
}
