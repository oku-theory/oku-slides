//! Builds the Swift bridge: OS voice, duplex audio, microphone permission (macOS only).

fn main() {
    println!("cargo:rerun-if-changed=swift/Package.swift");
    println!("cargo:rerun-if-changed=swift/Sources");
    // swift-rs is a build dependency on macOS only (Cargo.toml).
    #[cfg(target_os = "macos")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        swift_rs::SwiftLinker::new("13.4").with_package("OkuSpeech", "swift/").link();
        println!("cargo:rustc-link-lib=framework=AVFoundation");
    }
}
