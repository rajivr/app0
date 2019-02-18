#![feature(asm, generators, generator_trait)]
#![no_std]
#![allow(unused_must_use)]

#[allow(unused_imports)]
use tock;

use tock::{
    alarm::{Alarm, AlarmClient},
    button::{Button, ButtonClient},
    console_read::{ConsoleRead, ConsoleReadClient},
    console_write::{ConsoleWrite, ConsoleWriteClient, ConsoleWriteStr},
    has_callback_messages, has_client_messages,
    led::Led,
    reap_client_messages, syscalls,
    task::{DriverTask, DriverTaskClient, DriverTaskWithState},
};

use core::fmt::Write;
use core::ops::Generator;
use core::pin::Pin;
use core::str;

#[used]
#[no_mangle]
pub static mut DATA: [u32; 128] = [0xC0DEF00D; 128];

#[used]
#[no_mangle]
pub static mut BSS: [u32; 64] = [0x0; 64];

fn main_task() -> impl Generator<Yield = (), Return = ()> {
    || {
        let alarm = Alarm::new();
        let alarm_client = AlarmClient::new();

        let button = Button::new();
        let button_client = ButtonClient::new();

        let console_read = ConsoleRead::new();
        let console_read_client = ConsoleReadClient::new();

        let console_write = ConsoleWrite::new();
        let console_write_client = ConsoleWriteClient::new();

        let led = Led::new();

        console_write.initiate_write("\n".as_bytes());

        {
            // await!
            loop {
                if console_write_client.has_message() {
                    break;
                } else {
                    yield;
                }
            }
            console_write_client.reap_bytes_written_message();
            reap_client_messages();
        }

        // test simple printing to console
        for i in 0..5 {
            let mut w_buf: [u8; 64] = [0; 64];
            let w_offset: usize;

            let mut w = ConsoleWriteStr::new(&mut w_buf[..]);
            write!(w, "Hello world! {} \n", i).unwrap();
            w_offset = w.get_offset();

            console_write.initiate_write(&w_buf[..w_offset]);

            {
                // await!
                loop {
                    if console_write_client.has_message() {
                        break;
                    } else {
                        yield;
                    }
                }
                console_write_client.reap_bytes_written_message();
                reap_client_messages();
            }
        }

        console_write.initiate_write("Wrote 5 times\n\n".as_bytes());

        {
            // await!
            loop {
                if console_write_client.has_message() {
                    break;
                } else {
                    yield;
                }
            }
            console_write_client.reap_bytes_written_message();
            reap_client_messages();
        }

        console_write.initiate_write("Enter 5 characters or press button: ".as_bytes());

        {
            // await!
            loop {
                if console_write_client.has_message() {
                    break;
                } else {
                    yield;
                }
            }
            console_write_client.reap_bytes_written_message();
            reap_client_messages();
        }

        // test receiving input from multiple event sources - console and button
        console_read.initiate_read(5);

        button.enable_button_interrupt(0);
        button.initiate();

        {
            // select! - start
            loop {
                if console_read_client.has_message() || button_client.has_pressed_message() {
                    break;
                } else {
                    yield;
                }
            }
        }

        let mut w_buf: [u8; 64] = [0; 64];
        let mut r_buf: [u8; 64] = [0; 64];
        let mut w_offset: usize = 0;

        button_client.reap_get_pressed_data().map(|_| {
            console_write.initiate_write("\nReceived button press\n".as_bytes());

            // abort ongoing read
            console_read.abort();
        });

        console_read_client
            .reap_read_to_buffer(&mut r_buf[..5])
            .map(|_| {
                let mut w = ConsoleWriteStr::new(&mut w_buf[..]);
                write!(w, "\nReceived: {} \n", str::from_utf8(&r_buf[..5]).unwrap()).unwrap();
                w_offset = w.get_offset();
            })
            .map(|_| {
                // Extra map is needed to make rust borrow checker happy!
                console_write.initiate_write(&w_buf[..w_offset]);
            });

        {
            // select! - end
            //
            // reap uninterested messages
            reap_client_messages();
        }

        {
            // join!
            //
            // `console_read` is a special because of abort and state machine
            // semantics
            if console_read.is_active() {
                loop {
                    if console_read_client.has_message() {
                        break;
                    } else {
                        yield;
                    }
                }
            }

            loop {
                if console_write_client.has_message() {
                    break;
                } else {
                    yield;
                }
            }

            // join - end
            //
            // reap uninterested messages
            reap_client_messages();
        }

        console_write.initiate_write("\nPress Button to Turn On and Off LED\n".as_bytes());

        {
            // await!
            loop {
                if console_write_client.has_message() {
                    break;
                } else {
                    yield;
                }
            }
            console_write_client.reap_bytes_written_message();
            reap_client_messages();
        }

        let mut led_on_off = 0;

        // loop until led_on_off becomes 2.
        loop {
            {
                // select! - start
                loop {
                    if button_client.has_pressed_message() || console_write_client.has_message() {
                        break;
                    } else {
                        yield;
                    }
                }
            }

            console_write_client.reap_bytes_written_message();

            button_client.reap_get_pressed_data().map(|_| {
                if led_on_off == 0 {
                    led.on(0);
                    console_write.initiate_write("Turned LED On\n".as_bytes());
                    led_on_off += 1;
                } else if led_on_off == 1 {
                    led.off(0);
                    console_write.initiate_write("Turned LED Off\n".as_bytes());
                    led_on_off += 1;
                }
            });

            {
                // select! - end
                //
                // reap uninterested messages
                reap_client_messages();
            }

            if led_on_off == 2 {
                // Ensure write completes before breaking out of the loop
                if console_write.is_active() {
                    // await!
                    loop {
                        if console_write_client.has_message() {
                            break;
                        } else {
                            yield;
                        }
                    }
                    console_write_client.reap_bytes_written_message();
                    reap_client_messages();
                }

                break;
            }
        }

        console_write.initiate_write(
            "\nEnter 5 characters or press button or wait for 10 seconds:".as_bytes(),
        );

        {
            // await!
            loop {
                if console_write_client.has_message() {
                    break;
                } else {
                    yield;
                }
            }
            console_write_client.reap_bytes_written_message();
            reap_client_messages();
        }

        let mut stop_tic = 0;

        // clear r_buf
        r_buf.iter_mut().for_each(|x| *x = 0);

        console_read.initiate_read(5);

        alarm.get_tic().and_then(|current_tic| {
            let delay_tic = unsafe { alarm.millisecond_to_tic(10000) };
            stop_tic = current_tic + delay_tic;

            alarm.start(stop_tic)
        });
        alarm.initiate();

        // Select on console_read, alarm, button
        {
            // select! - start
            loop {
                if alarm_client.has_message()
                    || console_read_client.has_message()
                    || button_client.has_pressed_message()
                {
                    break;
                } else {
                    yield;
                }
            }
        }

        alarm_client.reap_get_data().map(|_| {
            console_write.initiate_write("\nAlarm expired\n".as_bytes());

            // abort ongoing read
            console_read.abort();
        });

        button_client.reap_get_pressed_data().map(|_| {
            console_write.initiate_write("\nReceived button press\n".as_bytes());

            // abort ongoing read
            console_read.abort();

            // stop alarm
            alarm.stop(stop_tic);
        });

        console_read_client
            .reap_read_to_buffer(&mut r_buf[..5])
            .map(|_| {
                let mut w = ConsoleWriteStr::new(&mut w_buf[..]);
                write!(w, "\nReceived: {} \n", str::from_utf8(&r_buf[..5]).unwrap()).unwrap();
                w_offset = w.get_offset();
            })
            .map(|_| {
                // Extra map is needed to make rust borrow checker happy!
                console_write.initiate_write(&w_buf[..w_offset]);

                // stop alarm
                alarm.stop(stop_tic);
            });

        {
            // select! - end
            //
            // reap uninterested messages
            reap_client_messages();
        }

        {
            // handle `console_read` abort
            if console_read.is_active() {
                loop {
                    if console_read_client.has_message() {
                        break;
                    } else {
                        yield;
                    }
                }
            }
        }

        {
            // await!
            loop {
                if console_write_client.has_message() {
                    break;
                } else {
                    yield;
                }
            }
            console_write_client.reap_bytes_written_message();
            reap_client_messages();
        }

        loop {
            {
                yield;
                reap_client_messages();
            }
        }
    }
}

#[inline(never)]
fn main() {
    unsafe {
        asm!("bkpt" :::: "volatile");
    }

    let alarm = Alarm::new();
    let mut alarm_task = unsafe { alarm.get_task() };

    let button = Button::new();
    let mut button_task = unsafe { button.get_task() };

    let console_read = ConsoleRead::new();
    let mut console_read_task = unsafe { console_read.get_task() };

    let console_write = ConsoleWrite::new();
    let mut console_write_task = unsafe { console_write.get_task() };

    let mut main_task_started = false;
    let mut main_task = main_task();

    loop {
        if alarm.has_message() {
            Pin::new(&mut alarm_task).resume();
        }

        if button.has_message() {
            Pin::new(&mut button_task).resume();
        }

        if console_read.has_message() {
            Pin::new(&mut console_read_task).resume();
        }

        if console_write.has_message() {
            Pin::new(&mut console_write_task).resume();
        }

        if main_task_started {
            // main task will *only* make progress if there are client messages
            if has_client_messages() {
                Pin::new(&mut main_task).resume();
            }
        } else {
            // start main task for the first time
            main_task_started = true;
            Pin::new(&mut main_task).resume();
        }

        if !has_callback_messages() {
            // Before calling yield, we need to *ensure* that there are *no*
            // incoming callback messages.
            //
            // There could however be incoming main_task messages which we might
            // not be interested in at the moment.
            syscalls::yieldk();
        }
    }

    // unsafe {
    //     asm!("bkpt" :::: "volatile");

    //     let x = DATA[0];
    //     let y = BSS[0];

    //     asm!("" :: "r"(x));
    //     asm!("" :: "r"(y));
    // }
}
