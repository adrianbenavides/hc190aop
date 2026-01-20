use clap::Parser;
use hc190aop::engine::PaymentEngine;
use hc190aop::reader::TransactionReader;
use hc190aop::writer::AccountWriter;
use miette::{IntoDiagnostic, Result};
use std::fs::File;
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input transactions CSV file
    input: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let file = File::open(cli.input).into_diagnostic()?;
    let reader = TransactionReader::new(file);

    let mut engine = PaymentEngine::new();

    for tx_result in reader.transactions() {
        match tx_result {
            Ok(tx) => {
                if let Err(e) = engine.process_transaction(tx) {
                    eprintln!("Error processing transaction: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error reading transaction: {}", e);
            }
        }
    }

    let stdout = io::stdout();
    let mut writer = AccountWriter::new(stdout.lock());
    writer.write_accounts(engine.accounts.into_values())?;

    Ok(())
}
