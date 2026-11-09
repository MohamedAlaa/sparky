use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=native/FoundationModel.swift");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") { return; }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let status = Command::new("xcrun").args(["swiftc", "-parse-as-library", "-O", "-module-cache-path"])
        .arg(out.join("swift-cache"))
        .arg("native/FoundationModel.swift").arg("-o").arg(out.join("foundation-model"))
        .status().expect("Apple's Swift compiler is required to build the on-device model bridge");
    assert!(status.success(), "Could not build Apple Foundation Model bridge");
}
