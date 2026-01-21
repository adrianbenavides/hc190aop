mod common;

#[test]
fn test_generate_simple_csv() {
    let output_path = std::path::PathBuf::from("test_generated.csv");
    common::generate_csv(&output_path, 5).expect("Failed to generate CSV");

    let content = std::fs::read_to_string(&output_path).expect("Failed to read file");
    // Header + 5 rows = 6 lines
    assert_eq!(content.lines().count(), 6);

    std::fs::remove_file(output_path).ok();
}

#[test]
fn test_generate_large_csv_distribution() {
    let output_path = std::path::PathBuf::from("test_dist_generated.csv");
    // Generate small amount but enough to see multiple clients
    common::generate_large_csv(&output_path, 1).expect("Failed to generate CSV");

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&output_path)
        .expect("Failed to open CSV");

    let mut client_ids = std::collections::HashSet::new();
    for result in reader.records() {
        let record = result.expect("Failed to read record");
        let client_id: u16 = record[1].parse().expect("Failed to parse client id");
        assert!((1..=50).contains(&client_id));
        client_ids.insert(client_id);
    }

    // With 1MB of data (~30k rows), we should definitely see most if not all 50 clients
    assert!(
        client_ids.len() > 1,
        "Should have seen more than one client ID"
    );
    assert!(
        client_ids.len() >= 40,
        "Should have seen most clients (at least 40/50)"
    );

    std::fs::remove_file(output_path).ok();
}
