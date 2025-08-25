use anyhow::{Context, Result};
use csv::Writer;
use engine::PaymentEngine;
use std::{env, fs::File, io::stdout};

pub mod account;
pub mod engine;
pub mod transaction;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        anyhow::bail!("Usage: {} transactions.csv", args[0]);
    }

    process_transactions(&args[1])?;

    Ok(())
}

fn process_transactions(input_path: &str) -> Result<()> {
    let file =
        File::open(input_path).with_context(|| format!("Failed to open file: {input_path}"))?;

    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file);

    let mut engine = PaymentEngine::new();

    for result in reader.deserialize() {
        match result {
            Ok(record) => {
                if let Err(e) = engine.process_transaction(record) {
                    eprintln!("Warning: Failed to process transaction: {e}");
                }
            }
            Err(_) => {
                // Silently ignore invalid CSV records as per requirements
                continue;
            }
        }
    }

    let mut writer = Writer::from_writer(stdout());

    for account in engine.get_accounts() {
        writer
            .serialize(account)
            .context("Failed to write output")?;
    }

    writer.flush().context("Failed to flush output")?;

    Ok(())
}
