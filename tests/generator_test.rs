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
fn test_generate_large_csv() {
    let output_path = std::path::PathBuf::from("test_large_generated.csv");
    // Generate 1MB
    common::generate_large_csv(&output_path, 1).expect("Failed to generate CSV");

    let size = std::fs::metadata(&output_path)
        .expect("Failed to get metadata")
        .len();
    assert!(size >= 1024 * 1024);

    std::fs::remove_file(output_path).ok();
}
