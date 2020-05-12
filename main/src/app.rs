use core::fmt::Write as _;
use embedded_hal::digital::v2::OutputPin as _;
use esp_idf_hal::{errors::EspError, gpio, i2c};
use ssd1306::{prelude::*, Builder};

use crate::freertos_task::{Cpu, CpuAffinity, CurrentTask, Task};
use crate::freertos_units::Duration;

#[no_mangle]
pub fn app_main() {
    EspError(unsafe { esp_idf_sys::nvs_flash_init() })
        .into_result()
        .unwrap();

    let oled_fn = move || {
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
        disp.reset(&mut oled_reset, &mut CurrentTask).unwrap();
        disp.init().unwrap();
        disp.clear().unwrap();
        disp.display_on(true).unwrap();

        let mut n = 0;
        loop {
            n += 1;
            disp.set_position(0, 0).unwrap();
            let _ = writeln!(&mut disp, "loop {}", n);
            CurrentTask::delay(Duration::ms(100));
        }
    };
    let _oled_task_h = Task::new()
        .name("oled_task")
        .stack_size(8192)
        .core_affinity(CpuAffinity::Cpu(Cpu::Pro))
        .start(oled_fn)
        .unwrap();

    let led_blink_fn = move || {
        let mut led_gpio = unsafe { gpio::OutputPin::new(25) };

        let mut n = 0;
        loop {
            n += 1;
            crate::println!(
                "loop {} stack_hw_mark: {}",
                n,
                CurrentTask::get_stack_high_water_mark()
            );

            led_gpio.set_high().unwrap();
            CurrentTask::delay(Duration::ms(100));
            led_gpio.set_low().unwrap();
            CurrentTask::delay(Duration::ms(100));
        }
    };

    let _led_blink_h = Task::new()
        .name("led_blink_task")
        .core_affinity(CpuAffinity::Cpu(Cpu::Pro))
        .start(led_blink_fn)
        .unwrap();

    crate::wifi::initialize_wifi();
}
