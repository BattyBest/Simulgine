//! AI-GENERATED TEST SUITE. Written by Claude, not by a human maintainer.
//! See tests/ai_generated/README.md.
//!
//! Shared helpers. Every case runs on its own thread with a timeout, so a panic or an infinite
//! loop in the library fails that one test instead of taking the whole run down.

mod crashes;
mod wrong_results;

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use simulgine::{
    compiler::runner::{compile_directory, tick_sim},
    run_free_const_expression, run_free_expression, run_simulgine,
};

pub const TIMEOUT: Duration = Duration::from_secs(10);

/// How a guarded run ended.
#[derive(Debug)]
pub enum Outcome<T> {
    Finished(T),
    Panicked(String),
    TimedOut,
}

impl<T: std::fmt::Debug> Outcome<T> {
    /// Unwraps a finished run, failing the test with a clear message on a panic or hang.
    #[track_caller]
    pub fn finished(self) -> T {
        match self {
            Outcome::Finished(x) => x,
            Outcome::Panicked(msg) => panic!("library panicked: {msg}"),
            Outcome::TimedOut => panic!("library did not finish within {TIMEOUT:?} (hang)"),
        }
    }
}

/// Runs `f` on a separate thread, catching panics and giving up after [`TIMEOUT`].
pub fn guarded<T, F>(f: F) -> Outcome<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let res = thread::spawn(f).join();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(TIMEOUT) {
        Ok(Ok(x)) => Outcome::Finished(x),
        Ok(Err(e)) => Outcome::Panicked(
            e.downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "<non-string panic>".into()),
        ),
        Err(_) => Outcome::TimedOut,
    }
}

/// Writes `src` into a fresh project directory as `main.sml` and returns the directory.
pub fn project(src: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "simulgine-ai-tests-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.sml"), src).unwrap();
    dir
}

/// Compiles `src`. Returns `Ok(())` or the error's message.
pub fn compile(src: &str) -> Outcome<Result<(), String>> {
    let dir = project(src);
    guarded(move || {
        compile_directory(&dir)
            .map(|_| ())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Compiles `src`, ticks `ticks` times, then reads each path (fields separated by ':', as in
/// `test_projects/*/expected.txt`) and returns its debug string, such as `[u64] 256`.
pub fn run(src: &str, ticks: usize, paths: &[&str]) -> Outcome<Vec<String>> {
    let dir = project(src);
    let paths: Vec<String> = paths.iter().map(|x| x.to_string()).collect();
    guarded(move || {
        let compiled = compile_directory(&dir).expect("program should compile");
        let mut sim = run_simulgine(&compiled);
        for _ in 0..ticks {
            tick_sim(&mut sim);
        }
        paths
            .iter()
            .map(|path| {
                let specs: Vec<_> = path.split(':').collect();
                let mut cur = sim.get_root();
                for spec in &specs[..specs.len() - 1] {
                    cur = cur
                        .get_field(&compiled, spec)
                        .unwrap()
                        .coerce_user_class()
                        .unwrap();
                }
                cur.get_field(&compiled, specs.last().unwrap())
                    .unwrap_or_else(|| panic!("no field {path}"))
                    .to_debug_string(Some(&compiled))
            })
            .collect()
    })
}

/// Evaluates a REPL expression with no project loaded. Returns the debug string or the error.
pub fn eval_const(expr: &str) -> Outcome<Result<String, String>> {
    let expr = expr.to_string();
    guarded(move || {
        run_free_const_expression(&expr)
            .map(|x| x.to_debug_string(None))
            .map_err(|e| format!("{e:?}"))
    })
}

/// Evaluates a REPL expression against a compiled, ticked project.
pub fn eval_in(src: &str, ticks: usize, expr: &str) -> Outcome<Result<String, String>> {
    let dir = project(src);
    let expr = expr.to_string();
    guarded(move || {
        let compiled = compile_directory(&dir).expect("program should compile");
        let mut sim = run_simulgine(&compiled);
        for _ in 0..ticks {
            tick_sim(&mut sim);
        }
        run_free_expression(&sim, &expr)
            .map(|x| x.to_debug_string(Some(&compiled)))
            .map_err(|e| format!("{e:?}"))
    })
}

/// Runs the REPL binary on `dir` with `stdin` and returns whether it exited cleanly within the
/// timeout. Used for failures (stack overflow, abort) that kill the whole process, which a thread
/// cannot contain.
pub fn repl_exits_cleanly(dir: &Path, stdin: &str) -> Result<(), String> {
    use std::io::Write;

    let mut child = Command::new(env!("CARGO_BIN_EXE_simulgine"))
        .arg(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // The child may already have died; a failed write is reported through its exit status.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("REPL exited abnormally: {status}"))
            };
        }
        if start.elapsed() > TIMEOUT {
            let _ = child.kill();
            return Err("REPL did not exit (hang)".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}
