mod compiler;
use std::{
    eprintln,
    io::{self, Write},
    path::Path,
};

use compiler::{
    runner::{
        compile_directory, run_free_const_expression, run_free_expression, run_simulgine, tick_sim,
    },
    simulgine_inst::SimulgineInst,
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

    if args.len() != 2 {
        eprintln!("USAGE: {} <directory of .sml files>", args[0]);
        return;
    }

    match compile_directory(Path::new(&args[1])) {
        Ok(x) => tui(run_simulgine(&x)),
        Err(x) => eprintln!("{}", x),
    }
}

fn tui(mut sim: SimulgineInst) {
    println!("Simulgine REPL Terminal");

    #[cfg(debug_assertions)]
    println!("{:?}", sim);

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
        if inp.chars().next().is_some_and(|x| x == '!') {
            match inp.split_whitespace().next().unwrap() {
                "!print" => println!("{:?}", sim),
                "!quit" => break,
                "!tick" => tick_sim(&mut sim),
                "!const" => {
                    if inp.len() <= 7 {
                        eprintln!("Usage:\n\t!const [expr]\n\tExamples:\n\t\t!const 3 [-> [u8] 3]\n\t\t!const 4 + 5 [-> [u8] 9]");
                        continue;
                    }
                    let expr = &inp[7..];
                    let res = run_free_const_expression(expr);
                    match res {
                        Ok(x) => println!("{}", x.to_debug_string(&sim.based)),
                        Err(x) => eprintln!("{}", x.to_string()),
                    }
                }
                _ => eprintln!("Unrecognized Command. Available: print, quit, tick, const [expr]"),
            }
            continue;
        }

        let expres = run_free_expression(&sim, inp);

        match expres {
            Ok(x) => println!("{}", x.to_debug_string(sim.based)),
            Err(x) => println!("{}", x),
        }
    }
}
