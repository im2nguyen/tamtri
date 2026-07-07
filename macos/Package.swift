// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "TamtriMac",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "Tamtri", targets: ["Tamtri"])
    ],
    targets: [
        .executableTarget(
            name: "Tamtri",
            dependencies: ["tamtri_coreFFI"],
            path: "Sources/Tamtri",
            exclude: ["Design/design-tokens.json"],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "../target/debug",
                    "-ltamtri_core",
                    "-Xlinker", "-rpath",
                    "-Xlinker", "../target/debug"
                ])
            ]
        ),
        .systemLibrary(
            name: "tamtri_coreFFI",
            path: "Sources/tamtri_coreFFI"
        ),
        .testTarget(
            name: "TamtriTests",
            dependencies: ["Tamtri"],
            path: "Tests/TamtriTests"
        )
    ]
)
