#![no_std]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

extern crate alloc;

use core::alloc::Layout;
use core::fmt::Write as _;
use core::panic::PanicInfo;

pub mod app;
pub mod errors;
pub mod freertos_task;
pub mod freertos_units;
pub mod wifi;

#[global_allocator]
static ALLOC: esp_idf_alloc::EspIdfAllocator = esp_idf_alloc::EspIdfAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    struct PrintF;

    impl core::fmt::Write for PrintF {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();

            let num_written = unsafe {
                esp_idf_sys::printf(
                    b"%.*s\0".as_ptr() as *const _,
                    bytes.len(),
                    bytes.as_ptr() as *const _,
                )
            };
            if num_written == bytes.len() as i32 {
                Ok(())
            } else {
                Err(core::fmt::Error)
            }
        }
    }

    if let Some(s) = info.payload().downcast_ref::<&str>() {
        let _ = writeln!(&mut PrintF, "panic msg: {}", s);
    }
    if let Some(args) = info.message() {
        let _ = writeln!(&mut PrintF, "{}", args);
    }
    if let Some(location) = info.location() {
        let _ = writeln!(
            &mut PrintF,
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
