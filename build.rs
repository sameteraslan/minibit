//! Build script for generating protobuf and Cap'n Proto schemas

fn main() {
    // Only run for benchmarks
    #[cfg(feature = "std")]
    {
        // Generate protobuf files if protoc is available
        if let Ok(_) = std::process::Command::new("protoc")
            .arg("--version")
            .output()
        {
            generate_protobuf_schema();
        }

        // Generate Cap'n Proto files if capnp is available
        if let Ok(_) = std::process::Command::new("capnp")
            .arg("--version")
            .output()
        {
            generate_capnp_schema();
        }
    }
}

#[cfg(feature = "std")]
fn generate_protobuf_schema() {
    use std::fs;
    use std::path::Path;

    // Create proto directory
    let proto_dir = "proto";
    if !Path::new(proto_dir).exists() {
        fs::create_dir_all(proto_dir).unwrap();
    }

    // Write trade.proto file
    let proto_content = r#"
syntax = "proto3";

package trade;

message TradeMessage {
    uint32 seq = 1;
    uint64 timestamp_ns = 2;
    int64 price = 3;
    uint32 quantity = 4;
    optional string symbol = 5;
    optional string note = 6;
}
"#;

    fs::write(format!("{}/trade.proto", proto_dir), proto_content).unwrap();

    // Generate Rust code (if protobuf-codegen is available)
    if let Err(_) = protobuf_codegen::Codegen::new()
        .pure()
        .out_dir("src/generated")
        .inputs(&[format!("{}/trade.proto", proto_dir)])
        .include(proto_dir)
        .run()
    {
        // Silently ignore if codegen fails
        eprintln!("Warning: Failed to generate protobuf code");
    }
}

#[cfg(feature = "std")]
fn generate_capnp_schema() {
    use std::fs;
    use std::path::Path;

    let capnp_dir = "capnp";
    if !Path::new(capnp_dir).exists() {
        fs::create_dir_all(capnp_dir).unwrap();
    }

    let capnp_content = r#"
@0x85150b117366d14b;

struct TradeMessage {
    seq @0 :UInt32;
    timestampNs @1 :UInt64;
    price @2 :Int64;
    quantity @3 :UInt32;
    symbol @4 :Text;
    note @5 :Text;
}
"#;
    fs::write(format!("{}/trade.capnp", capnp_dir), capnp_content).unwrap();

    // ✅ capnpc kullan
    if let Err(e) = capnpc::CompilerCommand::new()
        .src_prefix(capnp_dir)
        .file(format!("{}/trade.capnp", capnp_dir))
        .output_path("src/generated") // veya OUT_DIR, bkz. aşağıdaki not
        .run()
    {
        eprintln!("Warning: Failed to generate Cap'n Proto code: {e}");
    }

    // Build-cache tutarlılığı için:
    println!("cargo:rerun-if-changed={}/trade.capnp", capnp_dir);
}
