#![no_std]
#![feature(alloc_error_handler)]

use core::alloc::Layout;
use core::fmt::Write as _;
use core::panic::PanicInfo;

use embedded_hal::blocking::delay::DelayMs as _;
use embedded_hal::digital::v2::OutputPin as _;

use ssd1306::{prelude::*, Builder};

pub mod delay;
pub mod errors;
pub mod gpio;
pub mod i2c;
pub mod serial;

#[no_mangle]
pub fn app_main() {
    let mut buffered_printer = BufferedPrint::new();

    let oled_i2c_master = unsafe {
        i2c::Master::new(
            i2c::Port::Port0,
            i2c::PinConfig {
                pin_num: 4,
                pullup: true,
            },
            i2c::PinConfig {
                pin_num: 15,
                pullup: true,
            },
            400_000,
        )
    }
    .unwrap();
    let mut oled_reset = unsafe { gpio::OutputPin::new(16) };
    let mut disp: TerminalMode<_> = Builder::new().connect_i2c(oled_i2c_master).into();
    disp.reset(&mut oled_reset, &mut delay::FreeRtos).unwrap();
    disp.init().unwrap();
    disp.clear().unwrap();
    disp.display_on(true).unwrap();

    let mut led_gpio = unsafe { gpio::OutputPin::new(25) };

    let mut n = 0;
    loop {
        n += 1;
        let _ = writeln!(&mut buffered_printer, "loop {}", n);
        let _ = writeln!(&mut disp, "loop {}", n);

        led_gpio.set_high().unwrap();
        delay::FreeRtos.delay_ms(500u32);
        led_gpio.set_low().unwrap();
        delay::FreeRtos.delay_ms(500u32);
    }
}

struct BufferedPrint {
    buffer: [u8; 256],
    position: u8,
}

impl BufferedPrint {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; 256],
            position: 0,
        }
    }
}

impl core::fmt::Write for BufferedPrint {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.as_bytes().iter() {
            self.buffer[self.position as usize] = *byte;
            self.position += 1;
            if *byte == b'\n' || self.position >= 0xfe {
                unsafe { esp_idf_sys::printf(self.buffer.as_ptr() as *const _) };
                self.position = 0;
                for pos in self.buffer.iter_mut() {
                    *pos = 0;
                }
            }
        }

        Ok(())
    }
}

#[global_allocator]
static ALLOC: esp_idf_alloc::EspIdfAllocator = esp_idf_alloc::EspIdfAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut buffered_printer = BufferedPrint::new();

    if let Some(s) = info.payload().downcast_ref::<&str>() {
        let _ = writeln!(&mut buffered_printer, "panic msg: {}", s);
    }
    if let Some(location) = info.location() {
        let _ = writeln!(
            &mut buffered_printer,
            "panic location: {}:{}",
            location.file(),
            location.line(),
        );
    }
    unsafe { esp_idf_sys::abort() }
    unreachable!("post-abort")
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    unsafe { esp_idf_sys::abort() }
    unreachable!("post-abort")
}
