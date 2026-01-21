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
use std::sync::Arc;

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
        // We use Arc to share the opened DB handle during initialization
        let store = RocksDBStore::open(db_path).into_diagnostic()?;
        let shared_store = Arc::new(store);

        let as_store = shared_store.clone();
        let af = Box::new(move || Box::new((*as_store).clone()) as AccountStoreBox);

        let ts_store = shared_store;
        let tf = Box::new(move || Box::new((*ts_store).clone()) as TransactionStoreBox);

        PaymentEngine::new(af, tf)
    } else {
        // Use in-memory storage
        let af = Box::new(|| Box::new(InMemoryAccountStore::new()) as AccountStoreBox);
        let tf = Box::new(|| Box::new(InMemoryTransactionStore::new()) as TransactionStoreBox);

        PaymentEngine::new(af, tf)
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

    // Shutdown engine and collect final state
    let accounts = engine.shutdown().await?;

    // Output final state
    let stdout = io::stdout();
    let mut writer = AccountWriter::new(stdout.lock());
    writer.write_accounts(accounts).into_diagnostic()?;

    Ok(())
}
