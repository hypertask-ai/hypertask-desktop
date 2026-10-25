fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_RUN_NUMBER");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    for (input, output, fallback) in [
        ("GITHUB_RUN_NUMBER", "HT_RELEASE_SEQUENCE", "0"),
        ("GITHUB_SHA", "HT_RELEASE_SHA", "000000000000"),
    ] {
        let value = std::env::var(input).unwrap_or_else(|_| fallback.into());
        println!("cargo:rustc-env={output}={value}");
    }
}
