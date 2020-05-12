use core::fmt::Write;

pub struct PrintF;

impl Write for PrintF {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();

        // Call the `write` syscall directly to minimize the stack impact of `printf`.
        let num_written =
            unsafe { esp_idf_sys::write(1, bytes.as_ptr() as *const _, bytes.len() as u32) };
        if num_written == bytes.len() as i32 {
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}
