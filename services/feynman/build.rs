// In services/feynman/build.rs

fn main() {
    // Set an environment variable with the path to the project manifest (Cargo.toml)
    println!("cargo:rustc-env=CARGO_MANIFEST_DIR={}", std::env::var("CARGO_MANIFEST_DIR").unwrap());
}