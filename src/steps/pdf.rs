use async_trait::async_trait;
use anyhow::Result;
use image::DynamicImage;
use std::path::Path;
use tempfile::tempdir;
use leptess::LepTess;
use mupdf::{Document as MuDocument, Matrix, Colorspace};
use tracing::{debug, trace};
use rayon::prelude::*;
use crate::types::{ProcessError, Strategy, Config};
use crate::processor::{
    ProcessingStep,
    AsyncProcessor,
    format_extracted_data,
    format_ocr_data,
    optimize_image,
    clean_text,
    select_pages_to_process,
    is_meaningful_text
};
use crate::proto::processor::{Query, Attachment};

pub struct PDFProcessor;

#[async_trait]
impl AsyncProcessor for PDFProcessor {
    async fn process(&self, query: &mut Query, config: &Config) -> Result<(), ProcessError> {
        debug!("Starting PDF processing for file: {}", query.file_path);
        
        // Try to extract text directly from PDF
        let extracted_text = self.extract_text(Path::new(&query.file_path)).await?;
        trace!("Text extraction completed: {}", extracted_text.is_some());
        
        let has_extracted_text = if let Some(text) = extracted_text {
            let cleaned_text = clean_text(&text);
            if is_meaningful_text(&cleaned_text, config.ocr_quality_threshold) {
                debug!("Found meaningful text, adding to prompt parts");
                query.prompt_parts.push(format_extracted_data(&cleaned_text));
                true
            } else {
                debug!("Text not meaningful enough");
                false
            }
        } else {
            trace!("No text extracted");
            false
        };
        
        // Convert PDF to images
        debug!("Converting PDF to images");
        let images = self.convert_to_images(Path::new(&query.file_path), config).await?;
        trace!("Converted {} pages to images", images.len());
        
        // Process images in parallel
        debug!("Processing images");
        let results: Vec<_> = images.into_par_iter()
            .enumerate()
            .map(|(page_num, img)| {
                self.process_single_image(img, page_num, config, has_extracted_text)
            })
            .collect::<Result<Vec<_>, ProcessError>>()?;
        
        // Combine results
        let mut ocr_parts = Vec::new();
        let mut new_attachments = Vec::new();
        for (ocr_text, attachment) in results {
            if let Some(text) = ocr_text {
                ocr_parts.push(text);
            }
            new_attachments.push(attachment);
        }
        
        // Add OCR results and attachments
        query.prompt_parts.extend(ocr_parts);
        query.attachments.extend(new_attachments);
        trace!("Final state: {} prompt parts, {} attachments", 
            query.prompt_parts.len(), query.attachments.len());
        
        Ok(())
    }
}

impl ProcessingStep for PDFProcessor {
    fn required_for(&self) -> Vec<Strategy> {
        vec![Strategy::PDF]
    }

    fn name(&self) -> &'static str {
        "pdf_processor"
    }
}

impl PDFProcessor {
    async fn extract_text(&self, path: &Path) -> Result<Option<String>, ProcessError> {
        // Open PDF with mupdf
        let doc = MuDocument::open(path.to_str().ok_or_else(|| ProcessError::ExtractionFailed("Invalid path".to_string()))?)
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        let mut text = String::new();
        let total_pages = doc.page_count()
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        // Extract text from each page
        for page_num in 0..total_pages {
            if let Ok(page) = doc.load_page(page_num) {
                if let Ok(_) = page.bounds() {
                    if let Ok(page_text) = page.to_text() {
                        let cleaned_text = clean_text(&page_text);
                        if !cleaned_text.is_empty() {
                            text.push_str(&cleaned_text);
                            text.push('\n');
                        }
                    }
                }
            }
        }
        
        let text = text.trim().to_string();
        if !text.is_empty() {
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    fn process_single_image(
        &self,
        img: DynamicImage,
        page_num: usize,
        config: &Config,
        has_extracted_text: bool
    ) -> Result<(Option<String>, Attachment), ProcessError> {
        // Optimize image
        let (optimized, buffer) = optimize_image(&img, config.max_image_size_mb)?;
        
        // Create attachment
        let attachment = Attachment {
            page: (page_num + 1) as i32,
            data: buffer,
        };
        
        // Skip OCR if we already have meaningful text
        if has_extracted_text {
            return Ok((None, attachment));
        }
        
        // Create temp dir for OCR
        let temp_dir = tempdir().map_err(|e| ProcessError::IOError(e))?;
        
        // Initialize Tesseract
        let mut lt = Self::new_tesseract(&config.ocr_language)?;
        
        // Save to temporary file for OCR
        let temp_path = temp_dir.path().join(format!("page_{}.png", page_num + 1));
        optimized.save(&temp_path)
            .map_err(|e| ProcessError::ImageProcessingFailed(e.to_string()))?;
        
        // Perform OCR
        lt.set_image(&temp_path)
            .map_err(|e| ProcessError::OCRFailed(e.to_string()))?;
        
        let ocr_text = if let Ok(text) = lt.get_utf8_text() {
            let cleaned_text = clean_text(&text);
            if is_meaningful_text(&cleaned_text, config.ocr_quality_threshold) {
                Some(format_ocr_data(&cleaned_text, (page_num + 1) as u32))
            } else {
                None
            }
        } else {
            None
        };
        
        Ok((ocr_text, attachment))
    }

    async fn convert_to_images(
        &self,
        path: &Path,
        config: &Config
    ) -> Result<Vec<DynamicImage>, ProcessError> {
        // Open PDF with mupdf
        let doc = MuDocument::open(path.to_str().ok_or_else(|| ProcessError::ConversionFailed("Invalid path".to_string()))?)
            .map_err(|e| ProcessError::ConversionFailed(e.to_string()))?;
        
        let total_pages = doc.page_count()
            .map_err(|e| ProcessError::ConversionFailed(e.to_string()))?;
        
        // Determine which pages to convert
        let pages_to_convert = select_pages_to_process(total_pages, config);
        
        let mut images = Vec::new();
        let colorspace = Colorspace::device_rgb();
        
        // Convert selected pages to images sequentially
        for page_num in pages_to_convert {
            if let Ok(page) = doc.load_page(page_num) {
                // Create pixmap for rendering with reduced initial scale
                let pixmap = page.to_pixmap(
                    &Matrix::new_scale(1.5, 1.5),  // Back to 1.5 scale
                    &colorspace,
                    1.0,
                    false
                ).map_err(|e| ProcessError::ConversionFailed(e.to_string()))?;
                
                // Convert to DynamicImage
                let samples = pixmap.samples();
                let width = pixmap.width() as u32;
                let height = pixmap.height() as u32;
                let stride = pixmap.stride();
                let n = pixmap.n();
                
                // Pre-calculate buffer size and create with exact capacity
                let buffer_size = (width * height * 3) as usize;
                let mut rgb_data = Vec::with_capacity(buffer_size);
                unsafe { rgb_data.set_len(buffer_size); }
                
                // Process all pixels in a single pass
                let has_alpha = n == 4;
                let mut i = 0;
                
                for y in 0..height {
                    let row_start = y as usize * stride as usize;
                    for x in 0..width {
                        let pixel_start = row_start + x as usize * n as usize;
                        if pixel_start + (n as usize) - 1 >= samples.len() {
                            rgb_data[i] = 255;
                            rgb_data[i + 1] = 255;
                            rgb_data[i + 2] = 255;
                        } else {
                            let alpha = if has_alpha {
                                samples[pixel_start + 3] as f32 / 255.0
                            } else {
                                1.0
                            };
                            
                            for j in 0..3 {
                                let color = samples[pixel_start + j] as f32 * alpha + 255.0 * (1.0 - alpha);
                                rgb_data[i + j] = color as u8;
                            }
                        }
                        i += 3;
                    }
                }
                
                // Create the image
                let dynamic_image = image::RgbImage::from_raw(width, height, rgb_data)
                    .ok_or_else(|| ProcessError::ConversionFailed("Failed to create image".to_string()))?;
                
                images.push(DynamicImage::ImageRgb8(dynamic_image));
            }
        }
        
        Ok(images)
    }

    fn new_tesseract(lang: &str) -> Result<LepTess, ProcessError> {
        LepTess::new(None, lang)
            .map_err(|e| ProcessError::OCRFailed(e.to_string()))
    }
} 