//! Builds the Swift bridge to Apple's on-device model (macOS only).

fn main() {
    println!("cargo:rerun-if-changed=swift/Package.swift");
    println!("cargo:rerun-if-changed=swift/Sources");
    // swift-rs is a build dependency on macOS only (Cargo.toml).
    #[cfg(target_os = "macos")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        swift_rs::SwiftLinker::new("12.0").with_package("OkuModel", "swift/").link();
    }
}
