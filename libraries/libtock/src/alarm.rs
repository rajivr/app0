use core::ops::Generator;

use crate::result::{Error, Result};
use crate::syscalls::{command, subscribe, CallbackMessage};
use crate::task::{DriverTask, DriverTaskClient};

const DRIVER_NUM: usize = 0;

mod subscribe_num {
    pub const CALLBACK: usize = 0;
}

mod command_num {
    pub const PRESENT: usize = 0;
    pub const CLOCK_FREQUENCY: usize = 1;
    pub const TICK: usize = 2;
    pub const STOP: usize = 3;
    pub const START: usize = 4;
}

static mut ALARM_MESSAGE: Option<CallbackMessage> = None;

#[derive(Copy, Clone)]
pub enum AlarmClientMessage {
    Event(AlarmEventData),
}

static mut ALARM_CLIENT_MESSAGE: Option<AlarmClientMessage> = None;

extern "C" fn alarm_callback(arg0: usize, arg1: usize, arg2: usize, userdata: usize) {
    let cb_message = CallbackMessage::new(arg0, arg1, arg2, userdata);

    unsafe {
        ALARM_MESSAGE = Some(cb_message);
    }
}

#[derive(Copy, Clone)]
pub struct AlarmEventData {
    now: usize,
    expiration: usize,
}

impl AlarmEventData {
    pub fn new(now: usize, expiration: usize) -> AlarmEventData {
        AlarmEventData { now, expiration }
    }

    pub fn get_now(&self) -> usize {
        self.now
    }

    pub fn get_expiration(&self) -> usize {
        self.expiration
    }
}

pub struct Alarm;

impl Alarm {
    pub fn new() -> Alarm {
        Alarm
    }

    // Safety : This coroutine is called whenever there is an incoming callback
    //          message. When called, it *must* consume the incoming callback
    //          message before yielding.
    pub unsafe fn get_task(&self) -> impl Generator<Yield = (), Return = ()> + '_ {
        || loop {
            if let Some(cb_message) = ALARM_MESSAGE.take() {
                let now = cb_message.get_arg0();
                let expiration = cb_message.get_arg1();

                ALARM_CLIENT_MESSAGE = Some(AlarmClientMessage::Event(AlarmEventData::new(
                    now, expiration,
                )));
            }
            yield;
        }
    }

    pub fn initiate(&self) -> Result<()> {
        unsafe {
            subscribe(
                DRIVER_NUM,
                subscribe_num::CALLBACK,
                alarm_callback as *const _,
                0,
            )
            .map(|_| ())
        }
    }

    // Safety : Assumes `get_clock_frequency` will not fail
    pub unsafe fn millisecond_to_tic(&self, ms: usize) -> usize {
        let frequency = self.get_clock_frequency().unwrap();

        (ms / 1000) * frequency + (ms % 1000) * (frequency / 1000)
    }

    pub fn is_present(&self) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::PRESENT, 0, 0) }
    }

    pub fn get_clock_frequency(&self) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::CLOCK_FREQUENCY, 0, 0) }
    }

    pub fn get_tic(&self) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::TICK, 0, 0) }
    }

    pub fn stop(&self, tic: usize) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::STOP, tic, 0) }
    }

    pub fn start(&self, tic: usize) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::START, tic, 0) }
    }
}

impl DriverTask for Alarm {
    fn has_message(&self) -> bool {
        unsafe { ALARM_MESSAGE.is_some() }
    }
}

pub struct AlarmClient;

impl AlarmClient {
    pub fn new() -> AlarmClient {
        AlarmClient
    }

    pub fn reap_get_data(&self) -> Result<AlarmEventData> {
        unsafe {
            let a = ALARM_CLIENT_MESSAGE.clone();
            a.ok_or(Error::EINVAL).map(|x| match x {
                AlarmClientMessage::Event(d) => {
                    ALARM_CLIENT_MESSAGE = None;
                    d
                }
            })
        }
    }
}

impl DriverTaskClient for AlarmClient {
    fn has_message(&self) -> bool {
        unsafe { ALARM_CLIENT_MESSAGE.is_some() }
    }

    fn reap_message(&self) {
        unsafe {
            let a = ALARM_CLIENT_MESSAGE.clone();
            a.map(|_| {
                ALARM_CLIENT_MESSAGE = None;
            });
        }
    }
}
