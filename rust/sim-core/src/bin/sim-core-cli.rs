use std::io::{self, Read};
use std::process;

use serde::Serialize;
use sim_core::{advance_one_day, PhaseTimings, SimulationState, TickReport};

#[derive(Serialize)]
struct TickResult {
    state: SimulationState,
    report: TickReport,
    phases: PhaseTimings,
}

fn main() {
    let pretty = std::env::args().any(|arg| arg == "--pretty");
    let mut input = String::new();

    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("failed to read simulation state from stdin: {err}");
        process::exit(1);
    }

    let mut state: SimulationState = match serde_json::from_str(&input) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("failed to parse simulation state JSON: {err}");
            process::exit(1);
        }
    };

    let (report, phases) = advance_one_day(&mut state);
    let payload = TickResult { state, report, phases };

    let output = if pretty {
        serde_json::to_string_pretty(&payload)
    } else {
        serde_json::to_string(&payload)
    };

    match output {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("failed to serialize tick result: {err}");
            process::exit(1);
        }
    }
}
