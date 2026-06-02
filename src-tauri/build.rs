fn main() {
    tauri_build::build();
    build_injector_dll();
}

fn build_injector_dll() {
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("src-tauri has no parent");
    let injector_dir = project_root.join("injector-dll");

    // Re-run build script whenever injector-dll source changes
    println!("cargo:rerun-if-changed=../injector-dll/src");
    println!("cargo:rerun-if-changed=../injector-dll/Cargo.toml");

    if !injector_dir.exists() {
        println!("cargo:warning=injector-dll directory not found, skipping DLL build");
        return;
    }

    // Build the injector DLL crate
    let status = std::process::Command::new("cargo")
        .args(["build", "--lib"])
        .current_dir(&injector_dir)
        .status()
        .expect("Failed to invoke cargo for injector-dll");

    if !status.success() {
        panic!("injector-dll build failed — check injector-dll/src for errors");
    }

    // Locate built DLL and copy it next to the main executable
    let dll_src = injector_dir.join("target/debug/windows_island_injector_dll.dll");

    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("target"));

    let dll_dst = target_dir.join("debug/windows_island_injector_dll.dll");

    if dll_src.exists() {
        match std::fs::copy(&dll_src, &dll_dst) {
            Ok(_) => println!(
                "cargo:warning=injector DLL copied to {}",
                dll_dst.display()
            ),
            Err(e) => println!(
                "cargo:warning=injector DLL copy skipped (file in use?): {e}"
            ),
        };
    } else {
        println!(
            "cargo:warning=DLL not found at {} after build",
            dll_src.display()
        );
    }
}
