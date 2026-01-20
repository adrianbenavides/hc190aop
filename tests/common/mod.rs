use std::fs::File;
use std::path::Path;
use std::io::Error;

pub fn generate_csv(path: &Path, rows: usize) -> Result<(), Error> {
    let file = File::create(path)?;
    let mut wtr = csv::WriterBuilder::new().from_writer(file);

    wtr.write_record(&["type", "client", "tx", "amount"])?;

    for i in 1..=rows {
        wtr.write_record(&[
            "deposit",
            "1", 
            &i.to_string(),
            "1.0"
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn generate_large_csv(path: &Path, size_mb: usize) -> Result<(), Error> {
    let file = File::create(path)?;
    let mut wtr = csv::WriterBuilder::new().from_writer(file);
    wtr.write_record(&["type", "client", "tx", "amount"])?;

    let target_size = (size_mb * 1024 * 1024) as u64;
    let mut tx_id = 1;
    
    // Check size every 5000 rows to avoid syscall overhead
    loop {
        for _ in 0..5000 {
            wtr.write_record(&[
                "deposit",
                "1",
                &tx_id.to_string(),
                "1.0"
            ])?;
            tx_id += 1;
        }
        wtr.flush()?; // Flush to ensure file size is updated
        if std::fs::metadata(path)?.len() >= target_size {
            break;
        }
    }
    Ok(())
}
