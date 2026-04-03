//! Binary that generates TypeScript type definitions from Rust SyncEntity definitions.
//! Run with: cargo run --bin codegen

fn main() {
    // Ensure domain entities are linked in so inventory collects them
    domain::register_entities();

    let ts_content = sync_core::ts::generate_ts_file();

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("packages/client/generated");

    std::fs::create_dir_all(&out_dir).expect("Failed to create generated directory");
    let out_path = out_dir.join("sync.ts");
    std::fs::write(&out_path, ts_content).expect("Failed to write sync.ts");

    println!("Generated {}", out_path.display());
}
