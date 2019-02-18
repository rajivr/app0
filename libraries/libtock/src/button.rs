use core::ops::Generator;

use crate::result::{Error, Result};
use crate::syscalls::{command, subscribe, CallbackMessage};
use crate::task::{DriverTask, DriverTaskClient};

const DRIVER_NUM: usize = 3;

mod subscribe_num {
    pub const CALLBACK: usize = 0;
}

mod command_num {
    pub const NUM_BUTTONS: usize = 0;
    pub const ENABLE_INTERRUPT: usize = 1;
    pub const DISABLE_INTERRUPT: usize = 2;
    pub const CURRENT_STATE: usize = 2;
}

static mut BUTTON_MESSAGE: Option<CallbackMessage> = None;

#[derive(Copy, Clone)]
pub enum ButtonClientMessage {
    Event(ButtonEventData),
}

static mut BUTTON_CLIENT_PRESSED_MESSAGE: Option<ButtonClientMessage> = None;

static mut BUTTON_CLIENT_NOT_PRESSED_MESSAGE: Option<ButtonClientMessage> = None;

extern "C" fn button_callback(arg0: usize, arg1: usize, arg2: usize, userdata: usize) {
    let cb_message = CallbackMessage::new(arg0, arg1, arg2, userdata);

    unsafe {
        BUTTON_MESSAGE = Some(cb_message);
    }
}

#[derive(Copy, Clone)]
pub struct ButtonEventData {
    state: ButtonState,
    num: usize,
}

impl ButtonEventData {
    pub fn new(num: usize, state: ButtonState) -> ButtonEventData {
        ButtonEventData { num, state }
    }

    pub fn get_num(&self) -> usize {
        self.num
    }

    pub fn get_state(&self) -> ButtonState {
        self.state
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ButtonState {
    NotPressed,
    Pressed,
}

pub struct Button;

impl Button {
    pub fn new() -> Button {
        Button
    }

    // Safety : This coroutine is called whenever there is an incoming callback
    //          message. When called, it *must* consume the incoming callback
    //          message before yielding.
    pub unsafe fn get_task(&self) -> impl Generator<Yield = (), Return = ()> + '_ {
        || loop {
            if let Some(cb_message) = BUTTON_MESSAGE.take() {
                let button_num = cb_message.get_arg0();
                if cb_message.get_arg1() == 0 {
                    BUTTON_CLIENT_NOT_PRESSED_MESSAGE = Some(ButtonClientMessage::Event(
                        ButtonEventData::new(button_num, ButtonState::NotPressed),
                    ));
                } else {
                    BUTTON_CLIENT_PRESSED_MESSAGE = Some(ButtonClientMessage::Event(
                        ButtonEventData::new(button_num, ButtonState::Pressed),
                    ));
                };
            }
            yield;
        }
    }

    pub fn initiate(&self) -> Result<()> {
        unsafe {
            subscribe(
                DRIVER_NUM,
                subscribe_num::CALLBACK,
                button_callback as *const _,
                0,
            )
            .map(|_| ())
        }
    }

    pub fn get_num_buttons(&self) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::NUM_BUTTONS, 0, 0) }
    }

    pub fn enable_button_interrupt(&self, button_num: usize) -> Result<()> {
        unsafe { command(DRIVER_NUM, command_num::ENABLE_INTERRUPT, button_num, 0).map(|_| ()) }
    }

    pub fn disable_button_interrupt(&self, button_num: usize) -> Result<()> {
        unsafe { command(DRIVER_NUM, command_num::DISABLE_INTERRUPT, button_num, 0).map(|_| ()) }
    }

    pub fn get_button_state(&self, button_num: usize) -> Result<ButtonState> {
        unsafe {
            command(DRIVER_NUM, command_num::CURRENT_STATE, button_num, 0).map(|r| {
                if r == 0 {
                    ButtonState::NotPressed
                } else {
                    ButtonState::Pressed
                }
            })
        }
    }
}

impl DriverTask for Button {
    fn has_message(&self) -> bool {
        unsafe { BUTTON_MESSAGE.is_some() }
    }
}

pub struct ButtonClient;

impl ButtonClient {
    pub fn new() -> ButtonClient {
        ButtonClient
    }

    pub fn has_pressed_message(&self) -> bool {
        unsafe { BUTTON_CLIENT_PRESSED_MESSAGE.is_some() }
    }

    pub fn has_not_pressed_message(&self) -> bool {
        unsafe { BUTTON_CLIENT_NOT_PRESSED_MESSAGE.is_some() }
    }

    pub fn reap_pressed_message(&self) -> Result<()> {
        unsafe {
            let b = BUTTON_CLIENT_PRESSED_MESSAGE.clone();
            b.ok_or(Error::EINVAL).map(|_| {
                BUTTON_CLIENT_PRESSED_MESSAGE = None;
                ()
            })
        }
    }

    pub fn reap_not_pressed_message(&self) -> Result<()> {
        unsafe {
            let b = BUTTON_CLIENT_NOT_PRESSED_MESSAGE.clone();
            b.ok_or(Error::EINVAL).map(|_| {
                BUTTON_CLIENT_NOT_PRESSED_MESSAGE = None;
                ()
            })
        }
    }

    pub fn reap_get_pressed_data(&self) -> Result<ButtonEventData> {
        unsafe {
            let b = BUTTON_CLIENT_PRESSED_MESSAGE.clone();
            b.ok_or(Error::EINVAL).map(|x| match x {
                ButtonClientMessage::Event(d) => {
                    BUTTON_CLIENT_PRESSED_MESSAGE = None;
                    d
                }
            })
        }
    }

    pub fn reap_get_not_pressed_data(&self) -> Result<ButtonEventData> {
        unsafe {
            let b = BUTTON_CLIENT_NOT_PRESSED_MESSAGE.clone();
            b.ok_or(Error::EINVAL).map(|x| match x {
                ButtonClientMessage::Event(d) => {
                    BUTTON_CLIENT_NOT_PRESSED_MESSAGE = None;
                    d
                }
            })
        }
    }
}

impl DriverTaskClient for ButtonClient {
    fn has_message(&self) -> bool {
        self.has_pressed_message() || self.has_not_pressed_message()
    }

    fn reap_message(&self) {
        unsafe {
            let b = BUTTON_CLIENT_PRESSED_MESSAGE.clone();
            b.map(|_| {
                BUTTON_CLIENT_PRESSED_MESSAGE = None;
            });

            let b = BUTTON_CLIENT_NOT_PRESSED_MESSAGE.clone();
            b.map(|_| {
                BUTTON_CLIENT_NOT_PRESSED_MESSAGE = None;
            });
        }
    }
}
