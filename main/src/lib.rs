#![no_std]
#![feature(alloc_error_handler)]

use core::alloc::Layout;
use core::panic::PanicInfo;
use esp_idf_sys::bindings;

#[no_mangle]
pub fn app_main() {
    let led_gpio = bindings::gpio_config_t {
        pin_bit_mask: 1 << 25,
        intr_type: bindings::gpio_int_type_t_GPIO_INTR_DISABLE,
        mode: bindings::gpio_mode_t_GPIO_MODE_OUTPUT,
        pull_down_en: bindings::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
        pull_up_en: bindings::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
    };

    unsafe { bindings::gpio_config(&led_gpio as *const bindings::gpio_config_t) };

    unsafe { bindings::gpio_set_level(bindings::gpio_num_t_GPIO_NUM_25, 1) };
}

#[global_allocator]
static ALLOC: esp_idf_alloc::EspIdfAllocator = esp_idf_alloc::EspIdfAllocator;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { bindings::abort() }
    unreachable!("post-abort")
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    unsafe { bindings::abort() }
    unreachable!("post-abort")
}
