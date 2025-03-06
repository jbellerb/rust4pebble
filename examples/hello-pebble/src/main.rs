#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

use pebblesdk_sys::{
    APP_LOG_LEVEL_ERROR, FONT_KEY_GOTHIC_28_BOLD, GColor, GPoint, GRect, GSize,
    GTextAlignmentCenter, MINUTE_UNIT, TextLayer, TimeUnits, Window, WindowHandlers,
    app_event_loop, app_log, fonts_get_system_font, layer_add_child, layer_get_bounds, localtime,
    strftime, text_layer_create, text_layer_destroy, text_layer_get_layer,
    text_layer_set_background_color, text_layer_set_font, text_layer_set_text,
    text_layer_set_text_alignment, text_layer_set_text_color, tick_timer_service_subscribe, time,
    tm, window_create, window_destroy, window_get_root_layer, window_set_background_color,
    window_set_window_handlers, window_stack_push,
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let location = info.location();
    unsafe {
        app_log(
            APP_LOG_LEVEL_ERROR as u8,
            location.map(|loc| loc.file()).unwrap_or("???").as_ptr(),
            location.map(|loc| loc.line() as i32).unwrap_or(0),
            concat!("app '", env!("CARGO_BIN_NAME"), "' panicked\0").as_ptr(),
        );

        // Trigger a UsageFault exception so the kernel kills our process.
        asm!("udf 0", options(noreturn));
    }
}

static MAIN_WINDOW: AtomicPtr<Window> = AtomicPtr::new(null_mut());
static TIME_TEXT_LAYER: AtomicPtr<TextLayer> = AtomicPtr::new(null_mut());
static mut DISPLAY_TIME_BUFFER: [u8; 8] = [0; 8];

pub unsafe extern "C" fn main_window_load(_window: *mut Window) {
    unsafe {
        let main_window = MAIN_WINDOW.load(Ordering::Relaxed);

        let window_layer = window_get_root_layer(main_window);
        let bounds = layer_get_bounds(window_layer);

        window_set_background_color(main_window, GColor { argb: 0b11000000 });

        let time_text_layer = text_layer_create(GRect {
            origin: GPoint { x: 0, y: 59 },
            size: GSize {
                w: bounds.size.w,
                h: 36,
            },
        });

        text_layer_set_background_color(time_text_layer, GColor { argb: 0b00000000 });
        text_layer_set_text_color(time_text_layer, GColor { argb: 0b11111111 });
        text_layer_set_font(
            time_text_layer,
            fonts_get_system_font(FONT_KEY_GOTHIC_28_BOLD as *const u8),
        );
        text_layer_set_text_alignment(time_text_layer, GTextAlignmentCenter);

        TIME_TEXT_LAYER.store(time_text_layer, Ordering::Relaxed);
        layer_add_child(window_layer, text_layer_get_layer(time_text_layer));

        let current_time = time(null_mut());
        update_time(localtime(&raw const current_time));
    }
}

pub unsafe extern "C" fn main_window_unload(_window: *mut Window) {
    unsafe {
        text_layer_destroy(TIME_TEXT_LAYER.swap(null_mut(), Ordering::Relaxed));
    }
}

fn update_time(time: *mut tm) {
    unsafe {
        strftime(
            &raw mut DISPLAY_TIME_BUFFER as *mut u8,
            8,
            c"%H:%M".as_ptr(),
            time,
        );
        text_layer_set_text(
            TIME_TEXT_LAYER.load(Ordering::Relaxed),
            &raw mut DISPLAY_TIME_BUFFER as *const u8,
        );
    }
}

pub unsafe extern "C" fn tick_handler(tick_time: *mut tm, _units_changed: TimeUnits) {
    update_time(tick_time);
}

fn init() {
    unsafe {
        let main_window = window_create();

        window_set_window_handlers(
            main_window,
            WindowHandlers {
                load: Some(main_window_load),
                appear: None,
                disappear: None,
                unload: Some(main_window_unload),
            },
        );

        tick_timer_service_subscribe(MINUTE_UNIT, Some(tick_handler));

        MAIN_WINDOW.store(main_window, Ordering::Relaxed);
        window_stack_push(main_window, true);
    }
}

fn deinit() {
    unsafe { window_destroy(MAIN_WINDOW.swap(null_mut(), Ordering::Relaxed)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() {
    init();
    unsafe {
        app_event_loop();
    }
    deinit();
}
