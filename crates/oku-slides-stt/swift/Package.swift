// swift-tools-version:5.9
// Pont Swift du moteur de dictée d'Apple, lié en bibliothèque statique (build.rs).
import PackageDescription

let package = Package(
    name: "OkuSpeech",
    platforms: [.macOS(.v12)],
    products: [.library(name: "OkuSpeech", type: .static, targets: ["OkuSpeech"])],
    targets: [.target(name: "OkuSpeech", path: "Sources/OkuSpeech")]
)
