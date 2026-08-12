use std::{
    eprintln,
    io::{self, Write},
    path::Path,
};

use simulgine::{
    compiler::{
        runner::{compile_directory, tick_sim},
        simulgine_inst::Simulgine,
    },
    *,
};

fn main() {
    #[cfg(debug_assertions)]
    unsafe {
        backtrace_on_stack_overflow::enable();
    };

    // Spin up thrice as many threads as there are logical cores. We want maximum parallelism, other
    // processes be damned.
    let num_cpus = num_cpus::get();
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus * 3)
        .build_global()
        .unwrap();

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 2 {
        eprintln!("USAGE: {} [<directory of .sml files>]", args[0]);
        return;
    }

    tui(args.get(1).map(|x| x.as_str()));
}

fn tui(init_path: Option<&str>) {
    println!("Simulgine REPL Terminal");

    #[allow(unused_assignments)]
    let mut simb: Option<Simulgine> = None;
    let mut sim = None;

    let load_sim = |path: &str| -> Option<Simulgine> {
        match compile_directory(Path::new(path)) {
            Ok(x) => Some(x),
            Err(x) => {
                eprintln!("{}", x);
                None
            }
        }
    };

    if let Some(p) = init_path {
        simb = load_sim(p);
        sim = simb.as_ref().map(|x| run_simulgine(x));
        #[cfg(debug_assertions)]
        println!("{:?}", sim);
    }

    let mut inbuf = String::new();
    let stdin = io::stdin();
    loop {
        print!(">> ");
        io::stdout().flush().unwrap();
        inbuf.clear();
        let res = stdin.read_line(&mut inbuf).unwrap();
        let inp = inbuf.trim();

        if res == 0 {
            break;
        }

        // Built-in commands
        #[allow(unused_assignments)]
        if inp.chars().next().is_some_and(|x| x == '!') {
            match inp.split_whitespace().next().unwrap() {
                "!print" => println!("{:?}", sim),
                "!quit" => break,
                "!tick" => {
                    if let Some(sim) = &mut sim {
                        tick_sim(sim)
                    } else {
                        eprintln!("Can only tick the simulation when one in loaded.")
                    }
                }
                "!unload" => {
                    simb = None;
                    sim = None;
                }
                "!load" => {
                    if inp.len() <= 6 {
                        eprintln!("Usage:\n\t!load [path]\n\tExamples:\n\t\t!load test_examples/basic_company");
                        continue;
                    }
                    let path = &inp[6..];

                    simb = load_sim(path);
                    sim = simb.as_ref().map(|x| run_simulgine(x));
                    #[cfg(debug_assertions)]
                    println!("{:?}", sim);
                }
                "!const" => {
                    if inp.len() <= 7 {
                        eprintln!("Usage:\n\t!const [expr]\n\tExamples:\n\t\t!const 3 [-> [u8] 3]\n\t\t!const 4 + 5 [-> [u8] 9]");
                        continue;
                    }
                    let expr = &inp[7..];
                    let res = run_free_const_expression(expr);
                    match res {
                        Ok(x) => println!("{}", x.to_debug_string(sim.as_ref().map(|x| x.based))),
                        Err(x) => eprintln!("{}", x),
                    }
                }
                _ => eprintln!("Unrecognized Command. Available: print, quit, tick, const [expr], load [path], unload"),
            }
            continue;
        }

        let expres = match sim.as_ref() {
            Some(sim) => run_free_expression(sim, inp),
            None => run_free_const_expression(inp),
        };

        match expres {
            Ok(x) => println!("{}", x.to_debug_string(sim.as_ref().map(|x| x.based))),
            Err(x) => println!("{}", x),
        }
    }
}
