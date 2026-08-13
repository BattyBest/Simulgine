use std::{
    assert_eq,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use simulgine::{
    compiler::runner::{compile_directory, tick_sim, SimulgineErrors},
    run_simulgine,
};

#[test]
fn test_hellosim() -> Result<(), SimulgineErrors> {
    let path = "test_projects/hellosim";

    let compiled = compile_directory(Path::new(path))?;

    let mut sim = run_simulgine(&compiled);

    let expected_file = File::open(Path::new(path).join("expected.txt"))?;
    let file_buf = BufReader::new(expected_file).lines();

    for line in file_buf.map_while(Option::Some) {
        let line = line?;

        if line == "!tick" {
            tick_sim(&mut sim);
        } else {
            let [specifier, expected] = {
                let mut ls = line.split(";");
                [ls.next().unwrap(), ls.next().unwrap()]
            };

            let mut cur = sim.get_root();
            let specs: Vec<_> = specifier.split(":").collect();
            for spec in &specs[..specs.len() - 1] {
                cur = cur
                    .get_field(&compiled, spec)
                    .unwrap()
                    .coerce_user_class()
                    .unwrap();
            }

            let last = specs.last().unwrap();
            let last = cur.get_field(&compiled, *last).unwrap();

            assert_eq!(last.to_debug_string(Some(&compiled)), expected)
        }
    }

    Ok(())
}
