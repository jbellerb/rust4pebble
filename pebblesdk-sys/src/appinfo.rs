unsafe extern "C" {
    #[link_name = "__pbl_app_info"]
    pub static APP_INFO: PebbleProcessInfo;
}

include!(concat!(env!("OUT_DIR"), "/bindings_appinfo.rs"));
