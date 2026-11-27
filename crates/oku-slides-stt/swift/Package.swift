// swift-tools-version:5.9
// Swift bridge to the OS voice, the duplex audio and the microphone permission, linked as a static library (build.rs).
import PackageDescription

let package = Package(
    name: "OkuSpeech",
    platforms: [.macOS("13.4")],
    products: [.library(name: "OkuSpeech", type: .static, targets: ["OkuSpeech"])],
    targets: [.target(name: "OkuSpeech", path: "Sources/OkuSpeech")]
)
