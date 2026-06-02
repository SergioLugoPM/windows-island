fn main() {
    tauri_build::build();
    build_injector_dll();
}

fn build_injector_dll() {
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("src-tauri has no parent");
    let injector_dir = project_root.join("injector-dll");

    println!("cargo:rerun-if-changed=../injector-dll/src");
    println!("cargo:rerun-if-changed=../injector-dll/Cargo.toml");

    if !injector_dir.exists() {
        println!("cargo:warning=injector-dll directory not found, skipping");
        return;
    }

    // Match the host profile so debug builds stay fast and release builds are optimized
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let release_flag: &[&str] = if profile == "release" { &["--release"] } else { &[] };

    let status = std::process::Command::new("cargo")
        .arg("build")
        .arg("--lib")
        .args(release_flag)
        .current_dir(&injector_dir)
        .status()
        .expect("Failed to invoke cargo for injector-dll");

    if !status.success() {
        panic!("injector-dll build failed");
    }

    let dll_src = injector_dir
        .join("target")
        .join(&profile)
        .join("windows_island_injector_dll.dll");

    if !dll_src.exists() {
        println!("cargo:warning=DLL not found at {}", dll_src.display());
        return;
    }

    // 1. Copy next to the main exe (works for `cargo tauri dev`)
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("target"));

    let dll_next_to_exe = target_dir.join(&profile).join("windows_island_injector_dll.dll");
    match std::fs::copy(&dll_src, &dll_next_to_exe) {
        Ok(_)  => println!("cargo:warning=DLL → {}", dll_next_to_exe.display()),
        Err(e) => println!("cargo:warning=DLL copy skipped (in use?): {e}"),
    }

    // 2. Copy into src-tauri/resources/ so Tauri bundles it with the installer
    let resources_dir = manifest_dir.join("resources");
    std::fs::create_dir_all(&resources_dir).ok();
    let dll_resource = resources_dir.join("windows_island_injector_dll.dll");
    match std::fs::copy(&dll_src, &dll_resource) {
        Ok(_)  => println!("cargo:warning=DLL → resources/ (bundled)"),
        Err(e) => println!("cargo:warning=DLL resource copy failed: {e}"),
    }
}
