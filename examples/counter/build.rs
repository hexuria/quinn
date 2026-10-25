fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(format!("{manifest}/src/main.rs")).unwrap();
    let file = quinn_lower::lower_file(
        &source,
        quinn_lower::Shell {
            title: "Counter",
            width: 480,
            height: 360,
        },
    )
    .unwrap_or_else(|err| panic!("lower counter view: {err}"));
    let path = format!("{manifest}/src/ui_script.rs");
    let previous = std::fs::read_to_string(&path).unwrap_or_default();
    if previous != file {
        std::fs::write(&path, file).unwrap();
    }
    println!("cargo:rerun-if-changed=src/main.rs");
}
