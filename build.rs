// SPDX-License-Identifier: MIT OR Apache-2.0

fn main() {
    // Compile protobuf definitions
    if let Err(e) = tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/core/distributed/grpc")
        .compile(&["proto/transport.proto"], &["proto"])
    {
        eprintln!("Protobuf compilation failed: {}", e);
        // Don't exit here, as gRPC might be optional
    }
}
