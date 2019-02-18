use crate::result::Result;
use crate::syscalls::command;

const DRIVER_NUM: usize = 2;

mod command_num {
    pub const NUM_LEDS: usize = 0;
    pub const ON: usize = 1;
    pub const OFF: usize = 2;
    pub const TOGGLE: usize = 3;
}

pub struct Led;

impl Led {
    pub fn new() -> Led {
        Led
    }

    pub fn get_num_leds(&self) -> Result<usize> {
        unsafe { command(DRIVER_NUM, command_num::NUM_LEDS, 0, 0) }
    }

    pub fn on(&self, led_num: usize) -> Result<()> {
        unsafe { command(DRIVER_NUM, command_num::ON, led_num, 0).map(|_| ()) }
    }

    pub fn off(&self, led_num: usize) -> Result<()> {
        unsafe { command(DRIVER_NUM, command_num::OFF, led_num, 0).map(|_| ()) }
    }

    pub fn toggle(&self, led_num: usize) -> Result<()> {
        unsafe { command(DRIVER_NUM, command_num::TOGGLE, led_num, 0).map(|_| ()) }
    }
}
