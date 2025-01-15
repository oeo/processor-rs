use std::path::Path;
use image::{DynamicImage, imageops::FilterType};
use image::ImageEncoder;
use crate::proto::processor::Query;
use crate::types::{ProcessError, Strategy, Config};
use async_trait::async_trait;
use regex::Regex;
use lazy_static::lazy_static;
use tracing::{trace, warn, info};

lazy_static! {
    static ref WHITESPACE_RE: Regex = Regex::new(r"[ \t]+").unwrap();
    static ref REPEATED_CHARS_RE: Regex = Regex::new(r"[|Iil]{3,}").unwrap();
    static ref REPEATED_DOTS_RE: Regex = Regex::new(r"[.:]{3,}").unwrap();
    static ref REPEATED_DASHES_RE: Regex = Regex::new(r"[_-]{2,}").unwrap();
    static ref MULTIPLE_NEWLINES_RE: Regex = Regex::new(r"\n\s*\n").unwrap();
}

/// Clean and normalize text content
pub fn clean_text(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    // Normalize line endings
    let text = text
        .replace("\r\n", "\n")
        .replace("\r", "\n");

    // Clean up lines
    let text = text
        .split('\n')
        .map(|line| line.trim())
        .filter(|line| line.len() > 1)
        .collect::<Vec<_>>()
        .join("\n");

    // Clean up common artifacts
    let text = WHITESPACE_RE.replace_all(&text, " ");
    let text = REPEATED_CHARS_RE.replace_all(&text, "");
    let text = REPEATED_DOTS_RE.replace_all(&text, "...");
    let text = REPEATED_DASHES_RE.replace_all(&text, "--");
    let text = MULTIPLE_NEWLINES_RE.replace_all(&text, "\n\n");

    text.trim().to_string()
}

/// Optimize image specifically for OCR processing
pub fn optimize_image_for_ocr(img: &DynamicImage) -> Result<DynamicImage, ProcessError> {
    trace!("Optimizing image for OCR");
    let mut optimized = img.clone();
    
    // Convert to grayscale for better OCR
    optimized = optimized.grayscale();
    trace!("Converted to grayscale");
    
    // Increase contrast more aggressively
    optimized = optimized.adjust_contrast(1.5);
    trace!("Adjusted contrast");
    
    // Sharpen more aggressively for better text recognition
    optimized = optimized.unsharpen(2.0, 2);
    trace!("Applied sharpening");
    
    Ok(optimized)
}

/// Select which PDF pages to process based on total count
pub fn select_pages_to_process(total_pages: i32, _config: &Config) -> Vec<i32> {
    if total_pages <= 4 {
        // If 4 or fewer pages, process all
        (0..total_pages).collect()
    } else {
        // Process first 2 and last 2 pages
        let mut pages = Vec::new();
        pages.extend(0..2);
        pages.extend((total_pages-2)..total_pages);
        pages
    }
}

/// Validate and adjust spreadsheet range to prevent memory issues
pub fn validate_sheet_range(start_row: u32, start_col: u32, end_row: u32, end_col: u32) -> (u32, u32, u32, u32) {
    let max_rows = 1000;
    let max_cols = 100;
    
    let adjusted_end_row = if end_row - start_row > max_rows {
        start_row + max_rows
    } else {
        end_row
    };
    
    let adjusted_end_col = if end_col - start_col > max_cols {
        start_col + max_cols
    } else {
        end_col
    };
    
    (start_row, start_col, adjusted_end_row, adjusted_end_col)
}

#[async_trait]
pub trait AsyncProcessor: Send + Sync {
    async fn process(&self, query: &mut Query, config: &Config) -> Result<(), ProcessError>;
}

pub trait ProcessingStep: AsyncProcessor {
    fn required_for(&self) -> Vec<Strategy>;
    fn name(&self) -> &'static str;
}

pub struct Processor {
    config: Config,
    steps: Vec<Box<dyn ProcessingStep>>,
}

impl Processor {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            steps: Vec::new(),
        }
    }

    pub fn add_step<T: ProcessingStep + 'static>(&mut self, step: T) {
        self.steps.push(Box::new(step));
    }

    pub async fn process(&mut self, query: &mut Query) -> Result<Query, ProcessError> {
        // Get file extension from path
        let path = Path::new(&query.file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| ProcessError::UnsupportedFile("No file extension".to_string()))?;

        // Set file type to extension
        query.file_type = extension.to_string();
        
        // Determine strategy from extension
        let strategy = Strategy::from_extension(extension);
        query.strategy = strategy.to_string();

        // Process with appropriate steps
        for step in &self.steps {
            if step.required_for().contains(&strategy) {
                step.process(query, &self.config).await?;
            }
        }

        // Update metadata if present
        if let Some(metadata) = &mut query.metadata {
            metadata.completed_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            metadata.total_duration_ms = (metadata.completed_at - metadata.started_at) * 1000;
        }

        Ok(query.clone())
    }
}

pub fn format_text_data(text: &str) -> String {
    format!("<EXTRACTED_DATA>{}</EXTRACTED_DATA>", text)
}

pub fn format_csv_data(text: &str) -> String {
    format!("<EXTRACTED_DATA>{}</EXTRACTED_DATA>", text)
}

pub fn format_extracted_data(text: &str) -> String {
    format!("<EXTRACTED_DATA>{}</EXTRACTED_DATA>", text)
}

pub fn format_ocr_data(text: &str, page: u32) -> String {
    format!("<OCR PAGE={}>{}</OCR>", page, text)
}

pub fn format_ocr_text(text: &str, page: u32) -> String {
    format!("<OCR PAGE={}>{}</OCR>", page, text)
}

pub fn is_mostly_garbage(text: &str) -> bool {
    // Return true if text is empty or whitespace
    let text = text.trim();
    if text.is_empty() {
        trace!("ocr filter: rejected - empty text");
        return true;
    }

    // Too short to be meaningful
    if text.len() < 10 {
        trace!("ocr filter: rejected - too short (length: {})", text.len());
        return true;
    }

    // Calculate ratio of valid characters to total length
    let valid_chars: String = text.chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':' | '\'' | '"' | '(' | ')' | '-'))
        .collect();
    let valid_char_ratio = valid_chars.len() as f32 / text.len() as f32;
    if valid_char_ratio < 0.8 {
        trace!("ocr filter: rejected - low valid char ratio ({:.1}%)", valid_char_ratio * 100.0);
        return true;
    }

    // Check for repeated characters, excluding x, X and 0
    let has_repeats = text.chars()
        .collect::<Vec<_>>()
        .windows(5)
        .any(|w| {
            w[0] != 'x' && w[0] != 'X' && w[0] != '0' && 
            w.iter().all(|&c| c == w[0])
        });
    if has_repeats {
        trace!("ocr filter: rejected - (4+) repeated characters");
        return true;
    }

    // Check for too many special characters
    let special_chars: Vec<char> = text.chars()
        .filter(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace() && !matches!(c, '.' | ',' | ';' | ':' | '\'' | '"' | '(' | ')' | '-'))
        .collect();
    let special_ratio = special_chars.len() as f32 / text.len() as f32;
    if special_ratio > 0.15 {
        trace!("ocr filter: rejected - too many special chars ({:.1}%)", special_ratio * 100.0);
        return true;
    }

    // Split into words and filter out empty strings
    let words: Vec<&str> = text.split_whitespace().collect();
    
    // Check for reasonable word count
    if words.len() < 3 {
        trace!("ocr filter: rejected - too few words (count: {})", words.len());
        return true;
    }

    // Check for reasonable word lengths
    let long_words: Vec<&str> = words.iter().filter(|w| w.len() > 20).copied().collect();
    let long_ratio = long_words.len() as f32 / words.len() as f32;
    if long_ratio > 0.08 {
        trace!("ocr filter: rejected - too many long words ({:.1}%)", long_ratio * 100.0);
        return true;
    }

    // Check for reasonable average word length
    let avg_word_length = words.iter().map(|w| w.len()).sum::<usize>() as f32 / words.len() as f32;
    if avg_word_length < 2.0 || avg_word_length > 15.0 {
        trace!("ocr filter: rejected - unusual avg word length ({:.1})", avg_word_length);
        return true;
    }

    // Check for reasonable number of single-char words
    let single_char_words: Vec<&str> = words.iter().filter(|w| w.len() == 1).copied().collect();
    let single_char_ratio = single_char_words.len() as f32 / words.len() as f32;
    if single_char_ratio > 0.3 {
        trace!("ocr filter: rejected - too many single-char words ({:.1}%)", single_char_ratio * 100.0);
        return true;
    }

    // Check for reasonable number of word-like tokens
    let word_like_pattern = regex::Regex::new(r"^[A-Za-z]+$").unwrap();
    let word_like_count = words.iter()
        .filter(|w| w.len() > 1 && word_like_pattern.is_match(w))
        .count();
    let word_like_ratio = word_like_count as f32 / words.len() as f32;
    if word_like_ratio < 0.4 {
        trace!("ocr filter: rejected - too few word-like tokens ({:.1}%)", word_like_ratio * 100.0);
        return true;
    }

    trace!("ocr filter: accepted - passed all quality checks");
    false
}

pub fn is_meaningful_text(text: &str, _threshold: f32) -> bool {
    !is_mostly_garbage(text)
}

// Helper function to optimize image size
pub fn optimize_image(
    img: &image::DynamicImage,
    max_size_mb: u32
) -> Result<(image::DynamicImage, Vec<u8>), ProcessError> {
    info!("Starting image optimization:");
    info!("  Original dimensions: {}x{}", img.width(), img.height());
    info!("  Max size: {}MB", max_size_mb);
    
    let max_size_bytes = (max_size_mb * 1024 * 1024) as u64;
    let target_size_bytes = 2 * 1024 * 1024;  // Target 2MB per file for better quality
    let mut optimized = img.clone();
    
    // Only scale down if the image is larger than our target dimensions
    let (width, height) = {
        let max_dimension = 1600f32;
        let current_width = optimized.width() as f32;
        let current_height = optimized.height() as f32;
        let aspect = current_width / current_height;

        if current_width <= max_dimension && current_height <= max_dimension {
            // Image is already small enough
            (optimized.width(), optimized.height())
        } else if aspect > 1.0 {
            // Width is larger
            let new_width = max_dimension.min(current_width);
            let new_height = (new_width / aspect).round();
            (new_width as u32, new_height as u32)
        } else {
            // Height is larger
            let new_height = max_dimension.min(current_height);
            let new_width = (new_height * aspect).round();
            (new_width as u32, new_height as u32)
        }
    };

    if width < optimized.width() || height < optimized.height() {
        info!("  Scaling down from {}x{} to {}x{}", optimized.width(), optimized.height(), width, height);
        optimized = img.resize_exact(
            width,
            height,
            FilterType::Triangle  // Faster than Lanczos3 for initial resize
        );
        info!("  After resize: {}x{}", optimized.width(), optimized.height());
    } else {
        info!("  No resize needed, dimensions are within limits");
    }
    
    // Convert to RGB if needed
    optimized = match optimized {
        image::DynamicImage::ImageRgba8(_) => {
            info!("  Converting RGBA to RGB");
            optimized.to_rgb8().into()
        },
        _ => optimized,
    };
    
    // Try PNG compression
    let mut buffer = Vec::with_capacity((width * height * 3) as usize);
    let color_type = optimized.color();
    info!("  Color type: {:?}", color_type);

    info!("  Attempting PNG compression");
    image::codecs::png::PngEncoder::new(&mut buffer)
        .write_image(
            optimized.as_bytes(),
            width,
            height,
            color_type
        )
        .map_err(|e| {
            warn!("Initial compression failed: {}", e);
            ProcessError::ImageProcessingFailed(e.to_string())
        })?;
    info!("  Initial buffer size: {}", buffer.len());
    
    // If still too large, scale down further but maintain quality
    if buffer.len() as u64 > target_size_bytes {
        info!("  Buffer too large ({}), scaling down", buffer.len());
        let scale = 0.95f32.min((target_size_bytes as f32 / buffer.len() as f32).sqrt());
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;
        info!("  Scaling down to {}x{} (scale: {:.2})", new_width, new_height, scale);
        
        optimized = optimized.resize_exact(
            new_width,
            new_height,
            FilterType::Triangle  // Faster than Lanczos3 for final resize
        );

        buffer.clear();
        info!("  Cleared buffer for new attempt");
        let color_type = optimized.color();
        info!("  New color type: {:?}", color_type);

        image::codecs::png::PngEncoder::new(&mut buffer)
            .write_image(
                optimized.as_bytes(),
                new_width,
                new_height,
                color_type
            )
            .map_err(|e| {
                warn!("Compression failed: {}", e);
                ProcessError::ImageProcessingFailed(e.to_string())
            })?;
        info!("  New buffer size: {}", buffer.len());
    }

    if buffer.len() as u64 > max_size_bytes {
        warn!("Failed to optimize image to target size: {} > {}", buffer.len(), max_size_bytes);
        return Err(ProcessError::ImageProcessingFailed(
            "Could not optimize image to target size".to_string()
        ));
    }
    
    info!("Successfully optimized image to {} bytes", buffer.len());
    Ok((optimized, buffer))
} 