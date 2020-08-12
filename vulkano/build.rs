fn main() {
    let dir: std::path::PathBuf = [std::env::var("CARGO_MANIFEST_DIR").unwrap().as_str(),  "..",  "lib"].iter().collect();
    println!("cargo:rustc-link-search=native={}", dir.display());
}
