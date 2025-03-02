use std::env::{self, current_dir, var};
use std::fs::{write, DirBuilder};
use std::path::{Path, PathBuf};

fn main() {
    let target = var("TARGET").expect("get the rustc target");

    let out_dir = var("OUT_DIR").expect("get output directory from Cargo");
    let out_path = Path::new(&out_dir);

    let src_path = current_dir().expect("get the current directory");
    let sdk_path = locate_sdk(&target);
    let gen_path = generate_headers(out_path);

    bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", src_path.join("include").to_string_lossy()))
        .clang_arg(format!("-I{}", sdk_path.join("include").to_string_lossy()))
        .clang_arg(format!("-I{}", gen_path.to_string_lossy()))
        .use_core()
        .ctypes_prefix("cty")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("generate bindings with bindgen")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("write bindings file");
}

fn check_platform(target: &str, version: &str, platform: &str) {
    println!("cargo:rustc-check-cfg=cfg(pebble_sdk_version)");
    println!("cargo:rustc-check-cfg=cfg(pebble_sdk_platform, values(\"aplite\", \"basalt\", \"chalk\", \"diorite\"))");

    let expected = match platform {
        "aplite" => "thumbv7m-none-eabi",
        "basalt" | "chalk" | "diorite" => "thumbv7em-none-eabi",
        _ => panic!("Unknown platform: {}", platform),
    };
    assert_eq!(
        target, expected,
        "Support for {} must be built targeting {}",
        platform, expected
    );

    println!("cargo:rustc-cfg=pebble_sdk_version=\"{}\"", version);
    println!("cargo:rustc-cfg=pebble_sdk_platform=\"{}\"", platform);
}

fn locate_sdk(target: &str) -> PathBuf {
    let version = var("CARGO_CFG_PEBBLE_SDK_VERSION").unwrap_or("current".to_string());
    let platform = var("CARGO_CFG_PEBBLE_SDK_PLATFORM").unwrap_or("aplite".to_string());

    check_platform(target, &version, &platform);

    #[allow(deprecated)]
    let path = env::home_dir()
        .map(|dir| {
            dir.join(match var("CARGO_CFG_TARGET_OS").as_deref() {
                Ok("macos") => "Library/Application Support/Pebble SDK",
                _ => ".pebble-sdk",
            })
            .join("SDKs/")
            .join(&version)
            .join("sdk-core/pebble")
            .join(&platform)
        })
        .expect("determine SDK location");

    println!("cargo:rustc-link-search={}/lib", path.to_string_lossy());
    println!("cargo:rustc-link-lib=libpebble");

    path
}

fn generate_headers(out_path: &Path) -> PathBuf {
    let path = out_path.join("sdk_gen/include");
    let src_path = path.join("src");

    let mut builder = DirBuilder::new();
    builder.recursive(true);
    builder.create(&path).expect("create sdk_gen/include/");
    builder
        .create(&src_path)
        .expect("create sdk_gen/include/src/");

    write(path.join("message_keys.auto.h"), "").expect("write sdk_gen/include/message_keys.auto.h");
    write(src_path.join("resource_ids.auto.h"), "")
        .expect("write sdk_gen/include/src/resource_ids.auto.h");

    path
}
