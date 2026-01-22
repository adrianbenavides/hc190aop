use clap::Parser;
use hc190aop::application::engine::PaymentEngine;
use hc190aop::domain::ports::{AccountStoreBox, TransactionStoreBox};
use hc190aop::infrastructure::in_memory::{InMemoryAccountStore, InMemoryTransactionStore};
use hc190aop::infrastructure::rocksdb::RocksDBStore;
use hc190aop::interfaces::csv::account_writer::AccountWriter;
use hc190aop::interfaces::csv::transaction_reader::TransactionReader;
use miette::{IntoDiagnostic, Result};
use std::fs::File;
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input transactions CSV file
    input: PathBuf,

    /// Path to persistent database (optional). If provided, uses RocksDB.
    #[arg(long)]
    db_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let engine = if let Some(db_path) = cli.db_path {
        // Use persistent storage (RocksDB)
        let store = RocksDBStore::open(db_path).into_diagnostic()?;

        // Create boxed instances for each trait
        let as_store: AccountStoreBox = Box::new(store.clone());
        let ts_store: TransactionStoreBox = Box::new(store);

        PaymentEngine::new(as_store, ts_store)
    } else {
        // Use in-memory storage
        let as_store: AccountStoreBox = Box::new(InMemoryAccountStore::new());
        let ts_store: TransactionStoreBox = Box::new(InMemoryTransactionStore::new());

        PaymentEngine::new(as_store, ts_store)
    };

    // Process transactions
    let file = File::open(cli.input).into_diagnostic()?;
    let reader = TransactionReader::new(file);
    for tx_result in reader.transactions() {
        match tx_result {
            Ok(tx) => {
                if let Err(e) = engine.process_transaction(tx).await {
                    eprintln!("Error processing transaction: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error reading transaction: {}", e);
            }
        }
    }

    // Collect final state from engine
    let accounts = engine.into_results().await?;

    // Output final state
    let stdout = io::stdout();
    let mut writer = AccountWriter::new(stdout.lock());
    writer.write_accounts(accounts).into_diagnostic()?;

    Ok(())
}
