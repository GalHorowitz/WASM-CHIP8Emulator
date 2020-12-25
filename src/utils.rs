pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

extern crate web_sys;

/// Dummy type to implement `core::fmt::Write` on for `print!` macros
pub struct ConsoleWriter {
    buffer: String,
}

#[allow(dead_code)]
impl ConsoleWriter {
    pub fn new() -> Self {
        ConsoleWriter {
            buffer: String::new()
        }
    }

    pub fn flush_out(&mut self) {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&self.buffer));
        self.buffer.clear();
    }
}

// Based on `wasm-glue` package
impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, st: &str) -> core::fmt::Result {
        self.buffer.push_str(st);
        Ok(())
    }
}

// Magic macro code
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let mut console_writer = $crate::utils::ConsoleWriter::new();
        let _ = core::fmt::Write::write_fmt(&mut console_writer, format_args!($($arg)*));
        console_writer.flush_out();
    };
}
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        print!($($arg)*);
    };
}