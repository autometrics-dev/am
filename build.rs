use std::env::var;

fn main() {
    // https://stackoverflow.com/a/51311222/11494565
    println!("cargo:rustc-env=TARGET={}", var("TARGET").unwrap());
}
