fn main() {
    let mut dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    dir.push("..");
    dir.push("lib");
    println!("cargo:rustc-link-search=native={}", dir.display());
}
