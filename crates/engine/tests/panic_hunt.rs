//! Fuzz every command with adversarial params; a command must return `Err`, never panic or hang.
//! (Rule 9 in AGENTS.md.) Each command runs on its own thread with a timeout, so one bad command
//! can't wedge the run — panics are caught and hangs are reported. CI includes the slow sweep:
//! `cargo test -p photocraft-engine --test panic_hunt -- --include-ignored`.
use photocraft_engine::{EngineError, Session, command_specs};
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc;
use std::time::Duration;

fn fresh() -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width": 24, "height": 16})).unwrap();
    s.execute("layer.new.layer", json!({})).unwrap();
    s.execute("select.rect", json!({"x": 1, "y": 1, "width": 6, "height": 5})).unwrap();
    s
}

/// Run `id` with `p` on a fresh session, on a worker thread. Returns `Err` with a reason on panic
/// or timeout, `Ok` otherwise (whether the command returned Ok or a graceful Err).
fn run_guarded(id: &str, p: &Value) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let (id, p) = (id.to_string(), p.clone());
    std::thread::spawn(move || {
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut s = fresh();
            command_result(s.execute(&id, p))
        }));
        let _ = tx.send(r.unwrap_or_else(|_| Err("escaped panic".into())));
    });
    match rx.recv_timeout(Duration::from_secs(4)) {
        Ok(result) => result,
        Err(_) => Err("hung (>4s)".into()),
    }
}

fn command_result(result: photocraft_engine::Result<Value>) -> Result<(), String> {
    match result {
        Err(EngineError::Internal { cmd }) => Err(format!("internal panic in {cmd}")),
        _ => Ok(()), // Ordinary parameter/precondition errors are expected adversarial outcomes.
    }
}

#[test]
fn caught_internal_panics_fail_the_hunt() {
    assert!(command_result(Err(EngineError::Internal { cmd: "test.panic".into() })).is_err());
}

#[test]
fn graceful_input_errors_are_not_panics() {
    assert!(command_result(Err(EngineError::BadParams { cmd: "test.params".into(), msg: "bad input".into() })).is_ok());
    assert!(command_result(Ok(json!({}))).is_ok());
}

#[test]
#[ignore = "slow full-registry fuzz; run with --ignored"]
fn no_command_panics_or_hangs_on_adversarial_params() {
    let params: Vec<Value> = vec![
        json!({}),
        json!({"x": -9, "y": -9, "radius": 0, "amount": 0, "angle": 0, "opacity": 0, "scale": 0, "levels": 0, "tolerance": 0, "gamma": 0, "layer": 999999, "channel": 999, "index": 999999, "count": 0}),
        json!({"x": 1e5, "y": -1e5, "radius": 300, "amount": 300, "angle": 1e5, "opacity": -100, "scale": -5, "gamma": -1}),
        json!({"points": [], "from": [0, 0], "to": [0, 0], "stops": [], "matrix": [0, 0, 0, 0, 0, 0], "colors": [], "name": "", "mode": "", "style": ""}),
    ];
    let mut bad: Vec<String> = Vec::new();
    for spec in command_specs() {
        for p in &params {
            if let Err(why) = run_guarded(spec.id, p) {
                bad.push(format!("{} [{why}] <- {p}", spec.id));
                break;
            }
        }
    }
    assert!(bad.is_empty(), "{} commands panicked or hung:\n{}", bad.len(), bad.join("\n"));
}
