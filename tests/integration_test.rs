use std::path::{Path, PathBuf};
use processor_rs::{
    Config, Strategy, QueryOutput, Processor,
    steps::{TextProcessor, SpreadsheetProcessor, PDFProcessor, OfficeProcessor, ImageProcessor},
    proto::processor::{Query, QueryMetadata},
};

// Helper function to create a processor with all steps
fn create_full_processor() -> Processor {
    let mut processor = Processor::new(Config::default());
    processor.add_step(TextProcessor);
    processor.add_step(SpreadsheetProcessor);
    processor.add_step(PDFProcessor);
    processor.add_step(OfficeProcessor);
    processor.add_step(ImageProcessor);
    processor
}

// Helper function to create a query for a test file
fn create_test_query(file_path: &Path) -> Query {
    Query {
        file_type: String::new(), // Let processor determine this
        file_path: file_path.to_string_lossy().to_string(),
        strategy: String::new(), // Let processor determine this
        prompt_parts: Vec::new(),
        attachments: Vec::new(),
        system: "test".to_string(),
        prompt: String::new(),
        metadata: None,
    }
}

// Helper function to validate query results
fn validate_query_result(result: &Query, file_path: &Path) {
    assert!(!result.file_path.is_empty(), "File path should not be empty");
    
    // Get extension and expected strategy
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .expect("File should have extension");
    
    let expected_strategy = Strategy::from_extension(extension);
    assert_eq!(result.strategy, expected_strategy.to_string(), "Strategy should match file extension");
    assert_eq!(result.file_type, extension, "File type should match extension");
    
    match expected_strategy {
        Strategy::Text | Strategy::Spreadsheet | Strategy::Office => {
            assert!(!result.prompt_parts.is_empty(), "Should have extracted text");
            for part in &result.prompt_parts {
                assert!(
                    part.starts_with("<EXTRACTED_DATA>") || part.starts_with("<OCR PAGE="),
                    "Each part should be properly formatted"
                );
            }
        }
        Strategy::PDF => {
            assert!(
                !result.prompt_parts.is_empty() || !result.attachments.is_empty(),
                "Should have either extracted text or images"
            );
        }
        Strategy::Image => {
            assert!(!result.attachments.is_empty(), "Should have image attachments");
            assert_eq!(result.attachments[0].page, 1, "First page should be 1");
            assert!(!result.attachments[0].data.is_empty(), "Should have image data");
            assert!(!result.prompt_parts.is_empty(), "Should have OCR text");
            for part in &result.prompt_parts {
                assert!(part.starts_with("<OCR PAGE="), "Each part should be OCR text");
            }
        }
    }
}

#[tokio::test]
async fn test_text_files() {
    let mut processor = create_full_processor();
    let test_files = ["text-test-1.txt"];

    for file_name in test_files {
        let file_path = PathBuf::from("test_data").join(file_name);
        let mut query = create_test_query(&file_path);
        
        let result = processor.process(&mut query).await.unwrap();
        validate_query_result(&result, &file_path);
    }
}

#[tokio::test]
async fn test_spreadsheet_files() {
    let mut processor = create_full_processor();
    let test_files = ["spreadsheet-test-1.xlsx", "spreadsheet-test-2.csv"];

    for file_name in test_files {
        let file_path = PathBuf::from("test_data").join(file_name);
        let mut query = create_test_query(&file_path);
        
        let result = processor.process(&mut query).await.unwrap();
        validate_query_result(&result, &file_path);
    }
}

#[tokio::test]
async fn test_pdf_files() {
    let mut processor = create_full_processor();
    let test_files = ["pdf-test-1.pdf", "pdf-test-2.pdf"];

    for file_name in test_files {
        let file_path = PathBuf::from("test_data").join(file_name);
        let mut query = create_test_query(&file_path);
        
        let result = processor.process(&mut query).await.unwrap();
        validate_query_result(&result, &file_path);
    }
}

#[tokio::test]
async fn test_office_files() {
    let mut processor = create_full_processor();
    let test_files = [
        "office-test-1.docx",
        "office-test-2.pptx",
        "office-test-3.pptx",
        "office-test-4.rtf",
    ];

    for file_name in test_files {
        let file_path = PathBuf::from("test_data").join(file_name);
        let mut query = create_test_query(&file_path);
        
        let result = processor.process(&mut query).await.unwrap();
        validate_query_result(&result, &file_path);
    }
}

#[tokio::test]
async fn test_image_files() {
    let mut processor = create_full_processor();
    let test_files = ["image-test-1.jpeg"];

    for file_name in test_files {
        let file_path = PathBuf::from("test_data").join(file_name);
        let mut query = create_test_query(&file_path);
        
        let result = processor.process(&mut query).await.unwrap();
        validate_query_result(&result, &file_path);
    }
}

#[tokio::test]
async fn test_file_metadata() {
    let mut processor = create_full_processor();
    let test_file = "pdf-test-1.pdf";
    let file_path = PathBuf::from("test_data").join(test_file);
    let mut query = create_test_query(&file_path);
    
    // Add metadata for testing
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    query.metadata = Some(QueryMetadata {
        started_at,
        completed_at: 0,
        total_duration_ms: 0,
        original_file_size: std::fs::metadata(&file_path).unwrap().len() as i64,
        errors: Vec::new(),
        steps: Vec::new(),
    });
    
    // Add a small delay to ensure started_at and completed_at are different
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    
    let result = processor.process(&mut query).await.unwrap();
    
    // Validate metadata
    let metadata = result.metadata.expect("Should have metadata");
    assert!(metadata.completed_at > started_at, "Should have completion time");
    assert!(metadata.total_duration_ms > 0, "Should have processing duration");
    assert_eq!(
        metadata.original_file_size,
        std::fs::metadata(&file_path).unwrap().len() as i64,
        "Should have correct file size"
    );
} 