use core::fmt::Write;

pub struct PrintF;

impl Write for PrintF {
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
