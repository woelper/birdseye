use super::*;

use std::process::Command;


#[test]
fn walk() {
    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();

    Command::new("mkdir")
        .arg("-p")
        .arg("test/a/.b/c")
        .output()
        .unwrap();


    Command::new("dd")
        .arg("if=/dev/urandom")
        .arg("of=test/a/file_20m.20")
        .arg("bs=20MB")
        .arg("count=1")
        .output()
        .unwrap();


        // use walkdir::WalkDir;

     



 
}
