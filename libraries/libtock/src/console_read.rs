use core::ops::Generator;

use crate::result::{Error, Result, UsizeError};
use crate::syscalls::{allow, command, subscribe, CallbackMessage};
use crate::task::{DriverTask, DriverTaskClient, DriverTaskWithState};

const DRIVER_NUM: usize = 1;

mod allow_num {
    pub const READ: usize = 2;
}

mod subscribe_num {
    pub const READ: usize = 2;
}

mod command_num {
    pub const READ: usize = 2;
    pub const READ_ABORT: usize = 3;
}

static mut CONSOLE_READ_MESSAGE: Option<CallbackMessage> = None;

#[derive(Copy, Clone)]
pub enum ConsoleReadClientMessage {
    BytesRead(Result<usize>),
}

static mut CONSOLE_READ_CLIENT_MESSAGE: Option<ConsoleReadClientMessage> = None;

extern "C" fn console_read_callback(arg0: usize, arg1: usize, arg2: usize, userdata: usize) {
    let cb_message = CallbackMessage::new(arg0, arg1, arg2, userdata);

    unsafe {
        CONSOLE_READ_MESSAGE = Some(cb_message);
    }
}

#[derive(Copy, Clone)]
pub struct ReadsPending(usize);

#[derive(Copy, Clone)]
pub struct ReadsComplete(usize);

// Indicates if there is an ongoing read. If the ongoing read was aborted, then
// we go into Aborting and wait for the last callback. Once the read is
// complete, `CONSOLE_READ_STATE` is set to None and client message is sent.
#[derive(Copy, Clone)]
pub enum ConsoleReadState {
    Ongoing(ReadsPending, ReadsComplete),
    Aborting(ReadsPending, ReadsComplete),
}

static mut CONSOLE_READ_STATE: Option<ConsoleReadState> = None;

// Corresponds to kernel read buffer
static mut CONSOLE_READ_BUF: [u8; 64] = [0; 64];

pub struct ConsoleRead;

impl ConsoleRead {
    pub fn new() -> ConsoleRead {
        ConsoleRead
    }

    // Safety : This coroutine is called whenever there is an incoming callback
    //          message. When called, it *must* consume the incoming callback
    //          message before yielding.
    pub unsafe fn get_task(&self) -> impl Generator<Yield = (), Return = ()> + '_ {
        || loop {
            if let Some(cb_message) = CONSOLE_READ_MESSAGE.take() {
                let crs = CONSOLE_READ_STATE.clone();
                if let Some(x) = crs {
                    let y: UsizeError = cb_message.get_arg0().into();
                    match y.0 {
                        Some(e) => {
                            // Callback error
                            CONSOLE_READ_STATE = None;
                            CONSOLE_READ_CLIENT_MESSAGE =
                                Some(ConsoleReadClientMessage::BytesRead(Err(e)));
                        }
                        None => {
                            // No callback error
                            match x {
                                ConsoleReadState::Ongoing(rp, rc) => {
                                    let mut rp = rp.0;
                                    let mut rc = rc.0;

                                    rc += cb_message.get_arg1();
                                    rp -= cb_message.get_arg1();

                                    if rp == 0 {
                                        CONSOLE_READ_STATE = None;
                                        CONSOLE_READ_CLIENT_MESSAGE =
                                            Some(ConsoleReadClientMessage::BytesRead(Ok(rc)));
                                    } else {
                                        CONSOLE_READ_STATE = Some(ConsoleReadState::Ongoing(
                                            ReadsPending(rp),
                                            ReadsComplete(rc),
                                        ));
                                    }
                                }
                                ConsoleReadState::Aborting(_rp, rc) => {
                                    let mut rc = rc.0;

                                    rc += cb_message.get_arg1();

                                    CONSOLE_READ_STATE = None;
                                    CONSOLE_READ_CLIENT_MESSAGE =
                                        Some(ConsoleReadClientMessage::BytesRead(Ok(rc)));
                                }
                            }
                        }
                    }
                }
            }

            yield;
        }
    }

    pub fn initiate_read(&self, len: usize) -> Result<()> {
        unsafe {
            // is there an ongoing read
            if CONSOLE_READ_STATE.is_some() {
                return Err(Error::EBUSY);
            }

            // previous console read client message has not been consumed
            if ConsoleReadClient::new().has_message() {
                return Err(Error::EBUSY);
            }

            // invalid length
            if len > CONSOLE_READ_BUF.len() {
                return Err(Error::EINVAL);
            }

            allow(
                DRIVER_NUM,
                allow_num::READ,
                &CONSOLE_READ_BUF as *const u8 as *mut u8,
                len,
            )
            .and_then(|_| {
                subscribe(
                    DRIVER_NUM,
                    subscribe_num::READ,
                    console_read_callback as *const _,
                    0,
                )
            })
            .and_then(|_| command(DRIVER_NUM, command_num::READ, len, 0))
            .map(|_| {
                CONSOLE_READ_STATE = Some(ConsoleReadState::Ongoing(
                    ReadsPending(len),
                    ReadsComplete(0),
                ));
                ()
            })
        }
    }

    // `CONSOLE_READ_STATE` goes from `Ongoing(...)` to `Aborting(...)`
    pub fn abort(&self) -> Result<()> {
        unsafe {
            let c = CONSOLE_READ_STATE.clone();

            c.ok_or(Error::EINVAL).and_then(|x| match x {
                ConsoleReadState::Ongoing(rp, rc) => {
                    command(DRIVER_NUM, command_num::READ_ABORT, 0, 0).map(|_| {
                        CONSOLE_READ_STATE = Some(ConsoleReadState::Aborting(rp, rc));
                        ()
                    })
                }
                ConsoleReadState::Aborting(_, _) => Err(Error::EBUSY),
            })
        }
    }
}

impl DriverTask for ConsoleRead {
    fn has_message(&self) -> bool {
        unsafe { CONSOLE_READ_MESSAGE.is_some() }
    }
}

impl DriverTaskWithState for ConsoleRead {
    fn is_active(&self) -> bool {
        unsafe { CONSOLE_READ_STATE.is_some() }
    }
}

pub struct ConsoleReadClient;

impl ConsoleReadClient {
    pub fn new() -> ConsoleReadClient {
        ConsoleReadClient
    }

    pub fn reap_read_to_buffer(&self, buf: &mut [u8]) -> Result<()> {
        unsafe {
            let c = CONSOLE_READ_CLIENT_MESSAGE.clone();
            let res = c.ok_or(Error::EINVAL).and_then(|c| {
                match c {
                    ConsoleReadClientMessage::BytesRead(len) => {
                        len.and_then(|l| {
                            // We need to have a slice that can accomodate the buffer.
                            if l != buf.len() {
                                Err(Error::EINVAL)
                            } else {
                                buf.copy_from_slice(&CONSOLE_READ_BUF[..buf.len()]);
                                self.clear_console_read_buf();

                                Ok(())
                            }
                        })
                    }
                }
            });

            CONSOLE_READ_CLIENT_MESSAGE = None;

            res
        }
    }

    fn clear_console_read_buf(&self) {
        unsafe {
            &CONSOLE_READ_BUF.iter_mut().for_each(|x| *x = 0);
        }
    }
}

impl DriverTaskClient for ConsoleReadClient {
    fn has_message(&self) -> bool {
        unsafe { CONSOLE_READ_CLIENT_MESSAGE.is_some() }
    }

    fn reap_message(&self) {
        unsafe {
            let c = CONSOLE_READ_CLIENT_MESSAGE.clone();
            c.map(|_| {
                CONSOLE_READ_CLIENT_MESSAGE = None;
            });
        }
    }
}
