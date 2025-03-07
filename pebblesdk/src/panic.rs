use core::arch::asm;
use core::panic::PanicInfo;

use pebblesdk_sys::{app_log, APP_LOG_LEVEL_ERROR};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let location = info.location();
    unsafe {
        app_log(
            APP_LOG_LEVEL_ERROR as u8,
            location.map(|loc| loc.file()).unwrap_or("???\0").as_ptr(),
            location.map(|loc| loc.line() as i32).unwrap_or(0),
            c"app panicked".as_ptr(),
        );

        // Trigger a UsageFault exception so the kernel kills our process.
        asm!("udf 0", options(noreturn));
    }
}
