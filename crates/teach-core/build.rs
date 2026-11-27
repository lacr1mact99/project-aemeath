// Nhúng manifest (PerMonitorV2 DPI + asInvoker) cho bin `teach` bằng linker MSVC, không thêm crate.
fn main() {
    let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("teach.exe.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bin=teach=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bin=teach=/MANIFESTINPUT:{}", manifest.display());
    }
}
