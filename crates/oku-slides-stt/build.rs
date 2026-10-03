//! Compile le pont Swift du moteur d'Apple (macOS seulement).

fn main() {
    println!("cargo:rerun-if-changed=swift/Package.swift");
    println!("cargo:rerun-if-changed=swift/Sources");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    swift_rs::SwiftLinker::new("12.0").with_package("OkuSpeech", "swift/").link();
    for framework in ["Speech", "AVFoundation", "CoreMedia"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
