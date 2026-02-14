use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    if let Ok(protoc_path) = protoc_bin_vendored::protoc_bin_path() {
        unsafe {
            std::env::set_var("PROTOC", protoc_path);
        }
    }

    let proto_root = Path::new("../proto");
    let vendor_root = proto_root.join("vendor");
    let files = &[
        proto_root.join("eattura/sdi/v1/body.proto"),
        proto_root.join("eattura/sdi/v1/common.proto"),
        proto_root.join("eattura/sdi/v1/enums.proto"),
        proto_root.join("eattura/sdi/v1/errors.proto"),
        proto_root.join("eattura/sdi/v1/header.proto"),
        proto_root.join("eattura/sdi/v1/invoice.proto"),
        proto_root.join("eattura/sdi/v1/natura.proto"),
        proto_root.join("eattura/sdi/v1/options.proto"),
    ];

    println!("cargo:rerun-if-changed=../proto");

    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config.compile_protos(
        files,
        &[proto_root.to_path_buf(), vendor_root], // <--- Qui il trucco
    )?;

    Ok(())
}
