use std::path::Path;
use async_trait::async_trait;
use leptess::LepTess;
use tempfile::tempdir;
use tracing::{debug, trace};
use crate::processor::{ProcessingStep, AsyncProcessor, format_ocr_text, optimize_image, is_meaningful_text};
use crate::proto::processor::{Query, Attachment};
use crate::types::{Strategy, ProcessError, Config};

pub struct ImageProcessor;

#[async_trait]
impl AsyncProcessor for ImageProcessor {
    async fn process(&self, query: &mut Query, config: &Config) -> Result<(), ProcessError> {
        // Load image
        let img = image::open(Path::new(&query.file_path))
            .map_err(|e| ProcessError::ImageProcessingFailed(e.to_string()))?;

        // Optimize image and get buffer
        let (optimized, buffer) = optimize_image(&img, config.max_image_size_mb)?;

        // Add image as attachment
        query.attachments.push(Attachment {
            page: 1,
            data: buffer,
        });

        // Create temp dir for OCR
        let temp_dir = tempdir().map_err(|e| ProcessError::IOError(e))?;

        // Perform OCR
        let mut lt = LepTess::new(None, &config.ocr_language)
            .map_err(|e| ProcessError::OCRFailed(e.to_string()))?;

        // Save optimized image to temp file for OCR
        let temp_path = temp_dir.path().join("temp_ocr.png");
        optimized.save(&temp_path)
            .map_err(|e| ProcessError::OCRFailed(e.to_string()))?;

        // Set image for OCR
        lt.set_image(&temp_path)
            .map_err(|e| ProcessError::OCRFailed(e.to_string()))?;

        // Get OCR text
        let text = lt.get_utf8_text()
            .map_err(|e| ProcessError::OCRFailed(e.to_string()))?;

        trace!("OCR text extracted: {}", text);
        trace!("Text length: {}", text.trim().len());
        trace!("Word count: {}", text.trim().split_whitespace().count());

        // Only add meaningful text
        if is_meaningful_text(&text, config.ocr_quality_threshold) {
            debug!("Text is meaningful, adding to prompt parts");
            query.prompt_parts.push(format_ocr_text(&text, 1));
        } else {
            debug!("Text not meaningful enough");
        }

        // Temp dir will be automatically cleaned up when it goes out of scope
        // unless keep_temps is true
        if config.keep_temps {
            // If we want to keep temps, move the file to the configured temp directory
            let kept_path = config.temp_dir.join("temp_ocr.png");
            std::fs::rename(temp_path, kept_path)
                .map_err(|e| ProcessError::OCRFailed(e.to_string()))?;
        }

        Ok(())
    }
}

impl ProcessingStep for ImageProcessor {
    fn required_for(&self) -> Vec<Strategy> {
        vec![Strategy::Image]
    }

    fn name(&self) -> &'static str {
        "image_processor"
    }
}

// ... existing code ... 