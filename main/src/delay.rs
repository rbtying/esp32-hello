use crate::freertos_units::{Duration, DurationTicks};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use esp_idf_sys::{ets_delay_us, vTaskDelay};

/// Espressif Task Scheduler-based delay provider
pub struct Ets;

impl DelayUs<u32> for Ets {
    fn delay_us(&mut self, us: u32) {
        unsafe {
            ets_delay_us(us);
        }
    }
}

/// FreeRTOS-based delay provider
pub struct FreeRtos;

impl FreeRtos {
    fn delay<D: DurationTicks>(&self, d: D) {
        unsafe {
            vTaskDelay(d.to_ticks());
        }
    }
}

impl DelayMs<u32> for FreeRtos {
    fn delay_ms(&mut self, ms: u32) {
        // divide by tick length, rounding up
        self.delay(Duration::ms(ms));
    }
}

impl DelayMs<u8> for FreeRtos {
    fn delay_ms(&mut self, ms: u8) {
        self.delay(Duration::ms(ms as u32));
    }
}
