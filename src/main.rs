use clap::Parser;
use hc190aop::application::engine::PaymentEngine;
use hc190aop::domain::ports::{AccountStoreBox, TransactionStoreBox};
use hc190aop::infrastructure::in_memory::{InMemoryAccountStore, InMemoryTransactionStore};
#[cfg(feature = "storage-rocksdb")]
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
    #[arg(long, conflicts_with = "in_memory")]
    db_path: Option<PathBuf>,

    /// Force in-memory storage, even for large files.
    #[arg(long, conflicts_with = "db_path")]
    in_memory: bool,
}

const ROCKSDB_THRESHOLD_BYTES: u64 = 50 * 1024 * 1024; // 100 MB

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine storage type and handle temporary directory if needed
    let mut _temp_dir_handle: Option<tempfile::TempDir> = None;

    let (as_store, ts_store) = if let Some(db_path) = cli.db_path {
        // Explicit RocksDB
        #[cfg(feature = "storage-rocksdb")]
        {
            let store = RocksDBStore::open(db_path).into_diagnostic()?;
            (
                Box::new(store.clone()) as AccountStoreBox,
                Box::new(store) as TransactionStoreBox,
            )
        }
        #[cfg(not(feature = "storage-rocksdb"))]
        {
            let _ = db_path; // avoid unused variable warning
            eprintln!(
                "WARNING: Persistent storage requested via --db-path, but 'storage-rocksdb' feature is not enabled. Falling back to In-Memory storage."
            );
            (
                Box::new(InMemoryAccountStore::new()) as AccountStoreBox,
                Box::new(InMemoryTransactionStore::new()) as TransactionStoreBox,
            )
        }
    } else if cli.in_memory {
        // Explicit In-Memory
        (
            Box::new(InMemoryAccountStore::new()) as AccountStoreBox,
            Box::new(InMemoryTransactionStore::new()) as TransactionStoreBox,
        )
    } else {
        // Auto-selection based on file size
        let use_rocksdb = if let Ok(metadata) = std::fs::metadata(&cli.input) {
            if metadata.len() >= ROCKSDB_THRESHOLD_BYTES {
                #[cfg(feature = "storage-rocksdb")]
                {
                    eprintln!(
                        "Input file size ({:.2} MB) exceeds threshold. Using RocksDB storage.",
                        metadata.len() as f64 / (1024.0 * 1024.0)
                    );
                    true
                }
                #[cfg(not(feature = "storage-rocksdb"))]
                {
                    eprintln!(
                        "WARNING: Input file size ({:.2} MB) exceeds threshold, but 'storage-rocksdb' feature is not enabled. Falling back to In-Memory storage.",
                        metadata.len() as f64 / (1024.0 * 1024.0)
                    );
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if use_rocksdb {
            #[cfg(feature = "storage-rocksdb")]
            {
                let temp = tempfile::tempdir().into_diagnostic()?;
                let store = RocksDBStore::open(temp.path()).into_diagnostic()?;
                _temp_dir_handle = Some(temp);
                (
                    Box::new(store.clone()) as AccountStoreBox,
                    Box::new(store) as TransactionStoreBox,
                )
            }
            #[cfg(not(feature = "storage-rocksdb"))]
            {
                (
                    Box::new(InMemoryAccountStore::new()) as AccountStoreBox,
                    Box::new(InMemoryTransactionStore::new()) as TransactionStoreBox,
                )
            }
        } else {
            (
                Box::new(InMemoryAccountStore::new()) as AccountStoreBox,
                Box::new(InMemoryTransactionStore::new()) as TransactionStoreBox,
            )
        }
    };

    let engine = PaymentEngine::new(as_store, ts_store);

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
