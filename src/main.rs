use anyhow::{Context, Result};
use std::{env, fs::File};

mod transactions;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        anyhow::bail!("Usage: {} transactions.csv", args[0]);
    }

    Ok(())
}

fn process_transactions(input_path: &str) -> Result<()> {
    let file =
        File::open(input_path).with_context(|| format!("Failed to open file: {}", input_path))?;

    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file);

    Ok(())
}
