use std::env;
use std::path::Path;

fn main() {
    let target = env::var("TARGET").unwrap();

    if target.contains("linux") && Path::new("/usr/pkg").exists() {
        return println!("cargo:rustc-link-lib=static=ncursesw");
    }
}
