#![no_std]
#![no_main]

use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

#[allow(unused_imports)]
use pebblesdk::panic as _;

use pebblesdk::sys::{
    FONT_KEY_GOTHIC_28_BOLD, GColor, GContext, GDrawCommandImage, GPoint, GRect, GSize,
    GTextAlignmentCenter, Layer, MINUTE_UNIT, TextLayer, TimeUnits, Window, WindowHandlers,
    app_event_loop, fonts_get_system_font, gdraw_command_image_create_with_resource,
    gdraw_command_image_destroy, gdraw_command_image_draw, layer_add_child, layer_create,
    layer_destroy, layer_get_bounds, layer_set_update_proc, localtime, strftime, text_layer_create,
    text_layer_destroy, text_layer_get_layer, text_layer_set_background_color, text_layer_set_font,
    text_layer_set_text, text_layer_set_text_alignment, text_layer_set_text_color,
    tick_timer_service_subscribe, time, tm, window_create, window_destroy, window_get_root_layer,
    window_set_background_color, window_set_window_handlers, window_stack_push,
};

unsafe extern "C" {
    // TODO: Generate in build script
    static RESOURCE_ID_FERRIS_IMAGE: u32;
}

static MAIN_WINDOW: AtomicPtr<Window> = AtomicPtr::new(null_mut());
static TIME_TEXT_LAYER: AtomicPtr<TextLayer> = AtomicPtr::new(null_mut());
static FERRIS_CANVAS_LAYER: AtomicPtr<Layer> = AtomicPtr::new(null_mut());
static FERRIS_IMAGE: AtomicPtr<GDrawCommandImage> = AtomicPtr::new(null_mut());
static mut DISPLAY_TIME_BUFFER: [u8; 8] = [0; 8];

pub unsafe extern "C" fn main_window_load(_window: *mut Window) {
    unsafe {
        let main_window = MAIN_WINDOW.load(Ordering::Relaxed);

        let window_layer = window_get_root_layer(main_window);
        let bounds = layer_get_bounds(window_layer);

        window_set_background_color(main_window, GColor { argb: 0b11111001 });

        let time_text_layer = text_layer_create(GRect {
            origin: GPoint {
                x: 35,
                y: ((bounds.size.h - 66) * 5) / 8 - 18,
            },
            size: GSize { w: 60, h: 36 },
        });

        text_layer_set_background_color(time_text_layer, GColor { argb: 0b00000000 });
        text_layer_set_text_color(time_text_layer, GColor { argb: 0b11000000 });
        text_layer_set_font(
            time_text_layer,
            fonts_get_system_font(FONT_KEY_GOTHIC_28_BOLD as *const u8),
        );
        text_layer_set_text_alignment(time_text_layer, GTextAlignmentCenter);

        TIME_TEXT_LAYER.store(time_text_layer, Ordering::Relaxed);
        layer_add_child(window_layer, text_layer_get_layer(time_text_layer));

        let ferris_layer = layer_create(GRect {
            origin: GPoint {
                x: bounds.size.w - 115,
                y: bounds.size.h - 66,
            },
            size: GSize { w: 115, h: 66 },
        });
        layer_set_update_proc(ferris_layer, Some(update_proc));

        FERRIS_CANVAS_LAYER.store(ferris_layer, Ordering::Relaxed);
        layer_add_child(window_layer, ferris_layer);

        let current_time = time(null_mut());
        update_time(localtime(&raw const current_time));
    }
}

pub unsafe extern "C" fn main_window_unload(_window: *mut Window) {
    unsafe {
        text_layer_destroy(TIME_TEXT_LAYER.swap(null_mut(), Ordering::Relaxed));
        layer_destroy(FERRIS_CANVAS_LAYER.swap(null_mut(), Ordering::Relaxed));
        gdraw_command_image_destroy(FERRIS_IMAGE.swap(null_mut(), Ordering::Relaxed));
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

pub unsafe extern "C" fn update_proc(_layer: *mut Layer, ctx: *mut GContext) {
    let origin = GPoint { x: 0, y: 0 };
    unsafe { gdraw_command_image_draw(ctx, FERRIS_IMAGE.load(Ordering::Relaxed), origin) }
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

        let ferris_image = gdraw_command_image_create_with_resource(RESOURCE_ID_FERRIS_IMAGE);
        FERRIS_IMAGE.store(ferris_image, Ordering::Relaxed);

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
