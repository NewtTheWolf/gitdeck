fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Vendor protoc via protobuf-src so the build needs no system protoc.
    std::env::set_var("PROTOC", protobuf_src::protoc());

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../../proto/gitdeck/v1/gitdeck.proto"],
            &["../../proto"],
        )?;
    Ok(())
}
