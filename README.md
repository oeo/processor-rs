# processor-rs

A high-performance document processing pipeline written in Rust that supports multiple file formats and provides text extraction, OCR capabilities, and image processing.

## Features

- Multi-format document processing:
  - Text files (txt, csv)
  - Office documents (docx, rtf, xlsx, pptx)
  - PDF files
  - Images (jpg, png, gif, bmp, tiff, webp, heic)
  - Spreadsheets (csv, xls, xlsx)

- Advanced Processing Capabilities:
  - Text extraction from various document formats
  - OCR (Optical Character Recognition) for images and scanned documents
  - Spreadsheet data parsing and formatting
  - Image optimization and compression
  - PDF text extraction and image conversion
  - Intelligent text quality assessment
  - Advanced OCR filtering and validation

- Quality Control Features:
  - Comprehensive OCR quality validation:
    - Character validity ratio checks (80% minimum valid characters)
    - Special character ratio limits (15% maximum)
    - Word length analysis (1-20 characters per word)
    - Word count validation (minimum 3 words)
    - Average word length checks (2-15 characters)
    - Single-character word ratio limits (30% maximum)
    - Word-like token validation (40% minimum)
    - Repeated character detection
  - Text cleaning and normalization:
    - Whitespace normalization
    - Line break standardization
    - Special character cleanup
    - Artifact removal

- Performance Features:
  - Async processing pipeline
  - Configurable memory limits
  - Multi-threaded processing
  - Temporary file management
  - Progress tracking and metrics
  - Memory-efficient image handling
  - Optimized text processing

## Installation

### Prerequisites

- Rust toolchain (1.75.0 or later recommended)
- Tesseract OCR engine for image text extraction
- System dependencies:
  ```bash
  # Ubuntu/Debian
  sudo apt-get install leptonica-dev tesseract-ocr libtesseract-dev clang

  # macOS
  brew install tesseract leptonica
  ```

### Configuration

The processor supports various configuration options:
- OCR quality thresholds
- Maximum image size limits
- Memory usage constraints
- Temporary file handling
- Processing timeouts
- Thread count control

## Usage

Basic usage through command line:
```bash
./preprocess-document run <infile> [options]

Options:
  --uncompressed        Disable image compression
  --config <file>       Use custom config file
  --temp-dir <dir>      Specify temporary directory
  --keep-temps          Keep temporary files
  --verbose             Enable verbose logging
  --max-memory <mb>     Set maximum memory usage
  --timeout <seconds>   Set processing timeout
```

## Output Format

The processor generates structured output including:
- Extracted text with quality metrics
- OCR results with confidence scores
- Optimized image attachments
- Processing metadata and timing information
- Error and warning logs

## Error Handling

The processor includes comprehensive error handling for:
- Invalid file formats
- OCR processing failures
- Memory constraints
- Timeout conditions
- File system errors

## API Usage

```rust
use processor_rs::{Config, Processor, Strategy};
use processor_rs::steps::{TextProcessor, PDFProcessor, ImageProcessor};

async fn process_document() {
    // Initialize with custom config
    let config = Config::default();
    let mut processor = Processor::new(config);

    // Add processing steps
    processor.add_step(TextProcessor);
    processor.add_step(PDFProcessor);
    processor.add_step(ImageProcessor);

    // Process document
    let mut query = Query {
        file_path: "document.pdf".to_string(),
        file_type: "application/pdf".to_string(),
        strategy: Strategy::PDF.to_string(),
        // ... additional query parameters
    };

    let result = processor.process(&mut query).await.unwrap();
}
```

## Supported File Types

| Category | Extensions |
|----------|------------|
| Text | txt, csv |
| Office | doc, docx, docm, odt, rtf |
| Spreadsheets | xls, xlsx, xlsm, ods |
| Presentations | ppt, pptx, pptm, odp |
| Web | html, htm |
| Images | bmp, gif, jpg, jpeg, png, tiff, tif, webp, heic, heif |
| PDF | pdf |
