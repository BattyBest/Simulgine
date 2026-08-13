use std::path::Path;

use simulgine::{
    compiler::runner::{compile_directory, tick_sim, SimulgineErrors},
    run_simulgine,
};

#[test]
fn test_hellosim() -> Result<(), SimulgineErrors> {
    let path = "test_projects/hellosim";

    let compiled = compile_directory(Path::new(path))?;

    let mut sim = run_simulgine(&compiled);

    let root = sim.get_root();

    let hello_val = root
        .get_field(&compiled, "hello")
        .unwrap()
        .to_debug_string(Some(&compiled));

    assert_eq!(hello_val, "[string] \"\"");

    tick_sim(&mut sim);

    let root = sim.get_root();

    let hello_val = root
        .get_field(&compiled, "hello")
        .unwrap()
        .to_debug_string(Some(&compiled));

    assert_eq!(hello_val, "[string] \"Hello, world!\"");

    Ok(())
}
