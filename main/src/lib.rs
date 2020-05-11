#![no_std]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

extern crate alloc;

use core::alloc::Layout;
use core::panic::PanicInfo;

mod app;
pub mod freertos_task;
pub mod freertos_units;
mod print;
mod wifi;

pub use print::PrintF;

#[macro_export]
macro_rules! println {
    ($fmt:expr) => {
        #[allow(unused_import)]
        {
            use core::fmt::Write as _;
            let _ = writeln!(&mut $crate::PrintF, $fmt);
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        #[allow(unused_import)]
        {
            use core::fmt::Write as _;
            let _ = writeln!(&mut $crate::PrintF, $fmt, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! print {
    ($fmt:expr) => {
        #[allow(unused_import)]
        {
            use core::fmt::Write as _;
            let _ = write!(&mut PrintF, $fmt);
        }
     };
    ($fmt:expr, $($arg:tt)*) => {
        #[allow(unused_import)]
        {
            use core::fmt::Write as _;
            let _ = write!(&mut PrintF, $fmt, $($arg)*);
        }
    };
}

#[global_allocator]
static ALLOC: esp_idf_alloc::EspIdfAllocator = esp_idf_alloc::EspIdfAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        println!("panic msg: {}", s);
    }
    if let Some(args) = info.message() {
        println!("{}", args);
    }
    if let Some(location) = info.location() {
        println!("panic location: {}:{}", location.file(), location.line(),);
    }
    unsafe { esp_idf_sys::abort() }
    unreachable!("post-abort")
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    unsafe { esp_idf_sys::abort() }
    unreachable!("post-abort")
}
