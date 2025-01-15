# processor-rs

A high-performance document processing pipeline written in Rust that supports multiple file formats and provides text extraction, OCR capabilities, and image processing.

## Features

- Multi-format document processing:
  - Text files (txt, csv)
  - Office documents (docx, xlsx, pptx)
  - PDF files with advanced rendering
  - Images (jpg, png, gif, bmp, tiff, webp)
  - Spreadsheets (csv, xls, xlsx)

- Advanced Processing Capabilities:
  - Text extraction from various document formats
  - OCR (Optical Character Recognition) for images and scanned documents
  - Spreadsheet data parsing and formatting
  - PDF processing with 1.5x render scale for optimal quality
  - Intelligent text quality assessment
  - Advanced OCR filtering and validation

- Performance Optimizations:
  - Parallel image processing using rayon
  - Pre-allocated buffers with exact capacity
  - Single-pass pixel processing
  - Efficient alpha blending with white background
  - Fast image resizing using Triangle filter
  - Memory-efficient buffer reuse
  - Optimized PDF to image conversion
  - Smart page selection for large documents

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

- Architecture Features:
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
processor-rs run <infile> [options]

Options:
  --format <format>     Output format (json, html, protobuf)
  --config <file>       Use custom config file
  --temp-dir <dir>      Specify temporary directory
  --keep-temps          Keep temporary files
  --verbose            Enable verbose logging
  --max-memory <mb>    Set maximum memory usage
  --timeout <seconds>  Set processing timeout
```

## Output Formats

The processor supports multiple output formats:

### JSON
Structured output including:
- Extracted text with quality metrics
- OCR results with confidence scores
- Optimized image attachments
- Processing metadata and timing information
- Error and warning logs

### HTML
Clean, minimal visualization with:
- 13px monospace font throughout
- Scrollable text sections
- Right-aligned content
- Optimized image display
- Processing metadata
- Clear section separation

### Protobuf
Binary format for efficient machine processing.

## Error Handling

The processor includes comprehensive error handling for:
- Invalid file formats
- OCR processing failures
- Memory constraints
- Timeout conditions
- File system errors
- Buffer size mismatches
- Image conversion issues

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
        file_type: "pdf".to_string(),
        strategy: Strategy::PDF.to_string(),
        prompt_parts: Vec::new(),
        attachments: Vec::new(),
        system: "You are a helpful assistant.".to_string(),
        prompt: String::new(),
        metadata: Some(QueryMetadata::default()),
    };

    let result = processor.process(&mut query).await.unwrap();
}
```

## Supported File Types

| Category | Extensions |
|----------|------------|
| Text | txt, csv |
| Office | docx, xlsx |
| Spreadsheets | xls, xlsx |
| Images | bmp, gif, jpg, jpeg, png, tiff, webp |
| PDF | pdf |
