//! Estimate component costs from repeated pipeline-prefix measurements.
use serde::{Deserialize, Serialize};
use spars::{Model, Stage};
use std::{hint::black_box, time::Instant};

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    category: String,
    text: String,
}
#[derive(Serialize)]
struct Measurement {
    stage: &'static str,
    cumulative_seconds: f64,
}
#[derive(Serialize)]
struct Profile {
    group: String,
    documents_per_pass: usize,
    rounds: usize,
    warmup_passes: usize,
    measurements: Vec<Measurement>,
    shared_encoder_seconds: f64,
    note: &'static str,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 || !["short", "long"].contains(&args[3].as_str()) {
        return Err("usage: profile_stages MODEL CORPUS short|long ROUNDS".into());
    }
    let rounds: usize = args[4].parse()?;
    if rounds == 0 {
        return Err("rounds must be positive".into());
    }
    let model = Model::load(&args[1])?;
    let corpus: Corpus = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let texts: Vec<&str> = corpus
        .cases
        .iter()
        .filter(|case| (case.category == "long") == (args[3] == "long"))
        .map(|case| case.text.as_str())
        .collect();
    if texts.is_empty() {
        return Err("empty profile".into());
    }
    for text in &texts {
        black_box(model.process(text)?);
    }
    let stages = [
        ("tokenizer", Stage::Tokenizer),
        ("tagger", Stage::Tagger),
        ("parser", Stage::Parser),
        ("attribute_ruler_and_chunks", Stage::AttributeRuler),
        ("lemmatizer", Stage::Lemmatizer),
        ("ner", Stage::Ner),
    ];
    let mut times = [0.; 6];
    // Rotate stage order between passes to reduce order effects.
    for round in 0..rounds {
        for offset in 0..stages.len() {
            let index = (round + offset) % stages.len();
            for text in &texts {
                let start = Instant::now();
                let doc = black_box(model.process_until(black_box(text), stages[index].1)?);
                times[index] += start.elapsed().as_secs_f64();
                black_box(doc);
            }
        }
    }
    let docs = texts
        .iter()
        .map(|text| model.tokenize(text))
        .collect::<spars::Result<Vec<_>>>()?;
    let mut shared_encoder_seconds = 0.;
    for _ in 0..rounds {
        for doc in &docs {
            let start = Instant::now();
            let encoded = black_box(model.tok2vec(black_box(doc)));
            shared_encoder_seconds += start.elapsed().as_secs_f64();
            black_box(encoded);
        }
    }
    let profile = Profile {
        group: args[3].clone(), documents_per_pass: texts.len(), rounds, warmup_passes: 1,
        measurements: stages.iter().zip(times).map(|((stage, _), cumulative_seconds)| Measurement { stage, cumulative_seconds }).collect(),
        shared_encoder_seconds,
        note: "Prefix totals repeat earlier components. Differences estimate component costs and include timing noise. Shared encoder is measured separately on pretokenized documents. Load, corpus reading, and result destruction are excluded.",
    };
    println!("{}", serde_json::to_string_pretty(&profile)?);
    Ok(())
}
