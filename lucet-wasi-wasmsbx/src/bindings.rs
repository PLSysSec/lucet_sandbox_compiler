use lucet_module_wasmsbx::bindings::Bindings;

pub fn bindings() -> Bindings {
    Bindings::from_str(include_str!("../bindings.json")).expect("lucet-wasi-wasmsbx bindings.json is valid")
}

#[cfg(test)]
#[test]
fn test_bindings_parses() {
    let _ = bindings();
}
