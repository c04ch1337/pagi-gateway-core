fn main() {
    // Use a vendored protoc if the host doesn't have one installed.
    if std::env::var_os("PROTOC").is_none() {
        if let Ok(path) = protoc_bin_vendored::protoc_bin_path() {
            std::env::set_var("PROTOC", path);
        }
    }

    println!("cargo:rerun-if-env-changed=PROTOC");

    let proto_root = "../contracts";
    let protos = [
        format!("{proto_root}/agent.proto"),
        format!("{proto_root}/model.proto"),
        format!("{proto_root}/memory.proto"),
    ];

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&protos, &[proto_root])
        .expect("failed to compile protos");

    println!("cargo:rerun-if-changed=../contracts/agent.proto");
    println!("cargo:rerun-if-changed=../contracts/model.proto");
    println!("cargo:rerun-if-changed=../contracts/memory.proto");
}
