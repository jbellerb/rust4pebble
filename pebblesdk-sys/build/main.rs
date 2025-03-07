mod doxygen;

use std::env::{self, current_dir, var};
use std::fs::{write, DirBuilder, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use doxygen::ParseDoxygen;

use bindgen::Builder;

fn main() {
    let target = var("TARGET").expect("get the rustc target");

    let out_dir = var("OUT_DIR").expect("get output directory from Cargo");
    let out_path = Path::new(&out_dir);

    let src_path = current_dir().expect("get the current directory");
    let sdk_path = locate_sdk(&target);
    let gen_path = generate_headers(out_path);

    let include_paths = vec![src_path.join("include"), sdk_path.join("include"), gen_path];

    run("lib", "#include <pebble.h>", &include_paths, &out_path);
    run(
        "appinfo",
        "#include <pebble_process_info.h>",
        &include_paths,
        &out_path,
    );
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
    println!("cargo:rustc-link-lib=pebble");

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

fn run(name: &str, header: &str, include_paths: &Vec<PathBuf>, out_path: &Path) {
    let mut builder = Builder::default()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(ParseDoxygen))
        .use_core()
        .ctypes_prefix("cty")
        .prepend_enum_name(false);

    for path in include_paths {
        builder = builder.clang_arg("-I").clang_arg(path.to_string_lossy());
    }

    let header_path = out_path.join(format!("wrapper_{}.h", name));
    File::create(&header_path)
        .expect("create wrapper header")
        .write_all(header.as_bytes())
        .expect("write contents of wrapper header");

    builder
        .header(header_path.to_string_lossy())
        .generate()
        .expect("generate bindings with bindgen")
        .write_to_file(out_path.join(format!("bindings_{}.rs", name)))
        .expect("write bindings file");
}
