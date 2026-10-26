fn main() {
    println!("cargo:rerun-if-changed=windows.manifest");
    if std::env::var("TARGET").unwrap().ends_with("windows-msvc")
        && std::env::var_os("CARGO_FEATURE_LEGACY_WINDOWS_HEAP").is_none()
    {
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("windows.manifest");
        println!("cargo:rustc-link-arg-bin=ht-desktop=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bin=ht-desktop=/MANIFESTINPUT:{}", manifest.display());
    }
}
