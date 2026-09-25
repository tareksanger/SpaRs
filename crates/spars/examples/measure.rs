//! A measurement worker. Use tools/benchmark.py to record hardware and memory.
#[path = "measure/stats.rs"]
mod stats;
use serde_json::{json, Value};
use spars::Model;
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("usage: measure MODEL CORPUS load|short|long ROUNDS WARMUP".into());
    }
    let rounds: usize = args[4].parse()?;
    let warmup: usize = args[5].parse()?;
    if rounds == 0 || !["load", "short", "long"].contains(&args[3].as_str()) {
        return Err("rounds must be positive and profile must be load, short, or long".into());
    }
    let start = Instant::now();
    let model = Model::load(&args[1])?;
    let load_seconds = start.elapsed().as_secs_f64();
    if args[3] == "load" {
        println!("{}", json!({"load_seconds":load_seconds}));
        return Ok(());
    }
    let corpus: Value = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let texts: Vec<_> = corpus["cases"]
        .as_array()
        .ok_or("missing cases")?
        .iter()
        .filter(|c| (c["category"] == "long") == (args[3] == "long"))
        .map(|c| c["text"].as_str().ok_or("missing text"))
        .collect::<Result<_, _>>()?;
    if texts.is_empty() {
        return Err("empty benchmark profile".into());
    }
    for _ in 0..warmup {
        for text in &texts {
            black_box(model.process(text)?);
        }
    }
    let mut latencies = Vec::new();
    let mut token_count = 0;
    let mut elapsed = 0.;
    for _ in 0..rounds {
        for text in &texts {
            let start = Instant::now();
            let doc = black_box(model.process(black_box(text))?);
            let seconds = start.elapsed().as_secs_f64();
            elapsed += seconds;
            latencies.push(seconds);
            token_count += doc.tokens().len();
        }
    }
    latencies.sort_by(f64::total_cmp);
    let percentile =
        |p: f64| latencies[((latencies.len() as f64 * p).ceil() as usize).saturating_sub(1)];
    println!(
        "{}",
        json!({"load_seconds":load_seconds, "corpus_documents":texts.len(),
        "rounds":rounds,"warmup_passes":warmup,"measured_documents":latencies.len(),
        "measured_tokens":token_count,"inference_seconds":elapsed,
        "documents_per_second":latencies.len() as f64 / elapsed,
        "tokens_per_second":token_count as f64 / elapsed,
        "median_document_seconds":stats::median(&latencies).expect("nonempty measurements"),"p95_document_seconds":percentile(0.95)})
    );
    Ok(())
}
