use lucet_idl::{codegen, Backend, Config, Package};
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;
use tempfile::TempDir;

pub fn rust_test(package: &Package) {
    let config = Config {
        backend: Backend::RustHost,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    let gen_file = tempdir.path().join("lib.rs");

    codegen(
        package,
        &config,
        Box::new(File::create(gen_file.clone()).expect("create file")),
    )
    .expect("lucet_idl codegen");

    let cmd_rustc = Command::new("rustc")
        .arg("+stable")
        .arg(gen_file.clone())
        .arg("--allow=dead_code")
        .arg("--test")
        .arg("-o")
        .arg(tempdir.path().join("example"))
        .status()
        .expect("run rustc");

    if !cmd_rustc.success() {
        Command::new("cat")
            .arg(gen_file.clone())
            .status()
            .expect("debug output");
    }
    assert!(cmd_rustc.success(), "failure to compile generated code");
}

pub fn rust_wasm_codegen(package: &Package) -> Vec<u8> {
    let config = Config {
        backend: Backend::RustGuest,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    let idl_file = tempdir.path().join("idl.rs");
    let main_file = tempdir.path().join("main.rs");
    let wasm_file = tempdir.path().join("example.wasm");

    let mut main = File::create(main_file.clone()).expect("create main");
    main.write_all(
        b"#[allow(unused)]
mod idl;

fn main() {
    println!(\"hello, world\");
}
",
    )
    .expect("write contents of main");

    codegen(
        package,
        &config,
        Box::new(File::create(idl_file.clone()).expect("create file")),
    )
    .expect("lucet_idl codegen");

    let cmd_rustc = Command::new("rustc")
        .arg("+nightly")
        .arg(main_file.clone())
        .arg("--target=wasm32-wasi")
        .arg("--test")
        .arg("-o")
        .arg(wasm_file.clone())
        .status()
        .expect("run rustc");
    if !cmd_rustc.success() {
        Command::new("cat")
            .arg(idl_file.clone())
            .status()
            .expect("debug output");
    }
    assert!(cmd_rustc.success(), "failure to compile generated code");

    let mut wasm = File::open(wasm_file).expect("open wasm file");
    let mut buf = Vec::new();
    wasm.read_to_end(&mut buf).expect("read wasm file");
    buf
}

pub fn c_codegen(package: &Package) {
    let config = lucet_idl::Config {
        backend: lucet_idl::Backend::CGuest,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    codegen(
        package,
        &config,
        Box::new(File::create(tempdir.path().join("example.c")).expect("create file")),
    )
    .expect("lucet_idl codegen");

    let cmd_cc = Command::new("cc")
        .arg("--std=c99")
        .arg("-c")
        .arg(tempdir.path().join("example.c"))
        .arg("-o")
        .arg(tempdir.path().join("example.o"))
        .status()
        .expect("run cc");

    if !cmd_cc.success() {
        Command::new("cat")
            .arg(tempdir.path().join("example.c"))
            .status()
            .expect("debug output");
    }
    assert!(cmd_cc.success(), "failure to compile generated code");

    /*
    let cmd_run = Command::new(tempdir.path().join("example"))
        .status()
        .expect("run generated code");
    assert!(cmd_run.success(), "failure to run generated code");
    */
}