fn main() {
    // tauri-build compiles the .ico into resource.lib but does not declare it as a
    // build-script input, so editing the icon alone relinks a stale resource.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
