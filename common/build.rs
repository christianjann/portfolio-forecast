fn main() {
    let mut config = prost_build::Config::new();
    config.extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp");
    config.extern_path(".google.protobuf.Any", "::prost_types::Any");
    config.extern_path(".google.protobuf.NullValue", "::prost_types::NullValue");
    config.extern_path(".google.protobuf.Struct", "::prost_types::Struct");
    config.extern_path(".google.protobuf.Value", "::prost_types::Value");
    config.extern_path(".google.protobuf.ListValue", "::prost_types::ListValue");
    config
        .compile_protos(
            &["proto/client.proto"],
            &["proto/", "/usr/include"],
        )
        .unwrap();
    println!("cargo:rerun-if-changed=proto/client.proto");
}
