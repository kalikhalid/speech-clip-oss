fn main() {
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
