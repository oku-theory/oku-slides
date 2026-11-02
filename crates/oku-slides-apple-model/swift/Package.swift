// swift-tools-version:5.9
// Swift bridge to Apple's on-device language model, linked as a static library (build.rs).
import PackageDescription

let package = Package(
    name: "OkuModel",
    platforms: [.macOS(.v12)],
    products: [.library(name: "OkuModel", type: .static, targets: ["OkuModel"])],
    targets: [.target(name: "OkuModel", path: "Sources/OkuModel")]
)
