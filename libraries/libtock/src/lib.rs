#![feature(
    asm,
    core_intrinsics,
    generator_trait,
    generators,
    lang_items,
    naked_functions
)]
#![no_std]

pub mod alarm;
pub mod button;
pub mod console_read;
pub mod console_write;
pub mod entry_point;
pub mod lang_items;
pub mod led;
pub mod syscalls;
pub mod task;
pub mod unwind_symbols;

mod result;

use alarm::{Alarm, AlarmClient};
use button::{Button, ButtonClient};
use console_read::{ConsoleRead, ConsoleReadClient};
use console_write::{ConsoleWrite, ConsoleWriteClient};
use task::{DriverTask, DriverTaskClient};

pub fn reap_client_messages() {
    AlarmClient::new().reap_message();
    ButtonClient::new().reap_message();
    ConsoleReadClient::new().reap_message();
    ConsoleWriteClient::new().reap_message();
}

pub fn has_client_messages() -> bool {
    AlarmClient::new().has_message()
        || ButtonClient::new().has_message()
        || ConsoleReadClient::new().has_message()
        || ConsoleWriteClient::new().has_message()
}

pub fn has_callback_messages() -> bool {
    Alarm::new().has_message()
        || Button::new().has_message()
        || ConsoleRead::new().has_message()
        || ConsoleWrite::new().has_message()
}
