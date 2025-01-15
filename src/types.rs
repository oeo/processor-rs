use std::path::PathBuf;
use std::fmt;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub const SUPPORTED_BASE_FILE_EXTENSIONS: &[&str] = &[
    "csv", "txt",
    "doc", "docx", "docm", "odt", "rtf",
    "xls", "xlsx", "xlsm", "ods",
    "ppt", "pptx", "pptm", "odp",
    "html", "htm",
    "bmp", "gif", "jpg", "jpeg", "png", "tiff", "tif", "webp", "heic", "heif",
    "pdf"
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_image_size_mb: u32,
    pub max_rows: u32,
    pub max_cols: u32,
    pub ocr_language: String,
    pub ocr_quality_threshold: f32,
    pub temp_dir: PathBuf,
    pub threads: usize,
    pub timeout_seconds: u32,
    pub keep_temps: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_image_size_mb: 3,
            max_rows: 1000,
            max_cols: 100,
            ocr_language: "eng".to_string(),
            ocr_quality_threshold: 0.5,
            temp_dir: std::env::temp_dir(),
            threads: num_cpus::get(),
            timeout_seconds: 300,  // 5 minutes default
            keep_temps: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Strategy {
    Text,
    Spreadsheet,
    PDF,
    Office,
    Image,
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Strategy::Text => write!(f, "text"),
            Strategy::Spreadsheet => write!(f, "spreadsheet"),
            Strategy::PDF => write!(f, "pdf"),
            Strategy::Office => write!(f, "office"),
            Strategy::Image => write!(f, "image"),
        }
    }
}

impl Strategy {
    pub fn from_extension(extension: &str) -> Self {
        match extension.to_lowercase().as_str() {
            // Text files
            "txt" | "html" | "htm" => Strategy::Text,
            
            // Spreadsheets
            "csv" | "xls" | "xlsx" | "xlsm" | "ods" => Strategy::Spreadsheet,
            
            // PDF files
            "pdf" => Strategy::PDF,
            
            // Office documents
            "doc" | "docx" | "docm" | "odt" | "rtf" |
            "ppt" | "pptx" | "pptm" | "odp" => Strategy::Office,
            
            // Images
            "bmp" | "gif" | "jpg" | "jpeg" | "png" | 
            "tiff" | "tif" | "webp" | "heic" | "heif" => Strategy::Image,
            
            // Default to text for unknown extensions
            _ => Strategy::Text,
        }
    }
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Unsupported file type: {0}")]
    UnsupportedFile(String),
    #[error("Failed to extract text: {0}")]
    ExtractionFailed(String),
    #[error("Failed to convert document: {0}")]
    ConversionFailed(String),
    #[error("Failed to process document: {0}")]
    ProcessingFailed(String),
    #[error("Failed to perform OCR: {0}")]
    OCRFailed(String),
    #[error("Invalid processor")]
    InvalidProcessor,
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Image processing failed: {0}")]
    ImageProcessingFailed(String),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Progress {
    pub stage: String,
    pub percent: f32,
    pub current_file: Option<String>,
    pub memory_usage: u64,
    pub elapsed: std::time::Duration,
}

#[derive(Debug)]
pub struct Metrics {
    pub input_size: u64,
    pub output_size: u64,
    pub compression_ratio: f32,
    pub processing_time: std::time::Duration,
    pub peak_memory: u64,
    pub steps_completed: Vec<String>,
}

// JSON serialization wrapper for protobuf types
#[derive(Debug, Serialize)]
pub struct QueryOutput {
    pub file_type: String,
    pub file_path: String,
    pub strategy: String,
    pub prompt_parts: Vec<String>,
    pub attachments: Vec<AttachmentOutput>,
    pub system: String,
    pub prompt: String,
    pub metadata: Option<QueryMetadataOutput>,
}

#[derive(Debug, Serialize)]
pub struct AttachmentOutput {
    pub page: i32,
    pub data: String,
}

#[derive(Debug, Serialize)]
pub struct QueryMetadataOutput {
    pub started_at: i64,
    pub completed_at: i64,
    pub total_duration_ms: i64,
    pub original_file_size: i64,
    pub errors: Vec<String>,
    pub steps: Vec<ProcessingStepOutput>,
}

#[derive(Debug, Serialize)]
pub struct ProcessingStepOutput {
    pub name: String,
    pub duration_ms: i64,
    pub status: String,
    pub memory_mb: i64,
}

impl From<crate::proto::processor::Query> for QueryOutput {
    fn from(query: crate::proto::processor::Query) -> Self {
        Self {
            file_type: query.file_type,
            file_path: query.file_path,
            strategy: query.strategy,
            prompt_parts: query.prompt_parts,
            attachments: query.attachments.into_iter().map(Into::into).collect(),
            system: query.system,
            prompt: query.prompt,
            metadata: query.metadata.map(Into::into),
        }
    }
}

impl From<crate::proto::processor::Attachment> for AttachmentOutput {
    fn from(att: crate::proto::processor::Attachment) -> Self {
        Self {
            page: att.page,
            data: BASE64.encode(att.data),
        }
    }
}

impl From<crate::proto::processor::QueryMetadata> for QueryMetadataOutput {
    fn from(meta: crate::proto::processor::QueryMetadata) -> Self {
        Self {
            started_at: meta.started_at,
            completed_at: meta.completed_at,
            total_duration_ms: meta.total_duration_ms,
            original_file_size: meta.original_file_size,
            errors: meta.errors,
            steps: meta.steps.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::proto::processor::ProcessingStep> for ProcessingStepOutput {
    fn from(step: crate::proto::processor::ProcessingStep) -> Self {
        Self {
            name: step.name,
            duration_ms: step.duration_ms,
            status: step.status,
            memory_mb: step.memory_mb,
        }
    }
} 