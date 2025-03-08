use core::arch::asm;
use core::cmp::min;
use core::panic::PanicInfo;

use pebblesdk_sys::{app_log, appinfo::APP_INFO, APP_LOG_LEVEL_ERROR};

const FILE_BUFFER_LENGTH: usize = 64;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut file_buffer: [u8; FILE_BUFFER_LENGTH] = [0; FILE_BUFFER_LENGTH];
    let mut line = 0;
    if let Some(location) = info.location() {
        let file = location.file().as_bytes();
        let n = min(file.len(), FILE_BUFFER_LENGTH - 1);
        file_buffer[..n].copy_from_slice(&file[file.len() - n..file.len()]);
        line = location.line() as i32;
    } else {
        file_buffer[0] = '?' as u8;
    }

    unsafe {
        app_log(
            APP_LOG_LEVEL_ERROR as u8,
            file_buffer.as_ptr(),
            line,
            c"app '%s' panicked".as_ptr(),
            &raw const APP_INFO.name,
        );
    }

    // Trigger a UsageFault exception so the kernel kills our process.
    unsafe { asm!("udf 0", options(noreturn)) }
}
