use async_trait::async_trait;
use anyhow::Result;
use std::path::Path;
use std::io::{Read, BufReader};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use zip::ZipArchive;
use crate::types::{ProcessError, Strategy, Config};
use crate::processor::{ProcessingStep, AsyncProcessor, format_extracted_data, clean_text};
use crate::proto::processor::Query;

pub struct OfficeProcessor;

#[async_trait]
impl AsyncProcessor for OfficeProcessor {
    async fn process(&self, query: &mut Query, _config: &Config) -> Result<(), ProcessError> {
        // Try to extract text directly from the document
        let extracted_text = self.extract_text(Path::new(&query.file_path)).await?;
        if let Some(text) = extracted_text {
            let cleaned_text = clean_text(&text);
            if !cleaned_text.is_empty() {
                query.prompt_parts.push(format_extracted_data(&cleaned_text));
            }
        } else {
            // If no text was extracted, try reading as plain text
            match std::fs::read_to_string(&query.file_path) {
                Ok(content) => {
                    let cleaned_content = clean_text(&content);
                    if !cleaned_content.is_empty() {
                        query.prompt_parts.push(format_extracted_data(&cleaned_content));
                    }
                },
                Err(_) => return Err(ProcessError::ExtractionFailed("Failed to extract text".to_string())),
            }
        }

        Ok(())
    }
}

impl ProcessingStep for OfficeProcessor {
    fn required_for(&self) -> Vec<Strategy> {
        vec![Strategy::Office]
    }

    fn name(&self) -> &'static str {
        "office_processor"
    }
}

impl OfficeProcessor {
    async fn extract_text(&self, path: &Path) -> Result<Option<String>, ProcessError> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| ProcessError::InvalidFormat("No file extension".to_string()))?;
        
        match extension.to_lowercase().as_str() {
            "docx" => self.extract_docx(path),
            "rtf" => self.extract_rtf(path),
            "pptx" => self.extract_pptx(path),
            // Try plain text for other formats
            _ => Ok(None),
        }
    }

    fn extract_docx(&self, path: &Path) -> Result<Option<String>, ProcessError> {
        let file = std::fs::File::open(path)
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        let mut archive = ZipArchive::new(file)
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        // Find and read document.xml
        let mut content = String::new();
        if let Ok(mut doc) = archive.by_name("word/document.xml") {
            doc.read_to_string(&mut content)
                .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        } else {
            return Ok(None);
        }
        
        // Parse XML and extract text
        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);
        
        let mut text = String::new();
        let mut buf = Vec::new();
        let mut in_text = false;
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"w:t" => {
                    in_text = true;
                }
                Ok(Event::Text(e)) if in_text => {
                    if let Ok(t) = e.unescape() {
                        text.push_str(&t);
                        text.push(' ');
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"w:t" => {
                    in_text = false;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ProcessError::ExtractionFailed(e.to_string())),
                _ => (),
            }
            buf.clear();
        }
        
        let text = clean_text(&text);
        if !text.is_empty() {
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    fn extract_rtf(&self, path: &Path) -> Result<Option<String>, ProcessError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        // For RTF, replace \par with newlines and strip other RTF tags
        let text = content
            .replace("\\par", "\n")  // Replace paragraph markers with newlines
            .lines()
            .filter(|line| !line.starts_with("{") && !line.starts_with("}") && !line.starts_with("\\"))
            .collect::<Vec<_>>()
            .join("\n");
        
        let text = clean_text(&text);
        if !text.is_empty() {
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    fn extract_pptx(&self, path: &Path) -> Result<Option<String>, ProcessError> {
        let file = std::fs::File::open(path)
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        let mut archive = ZipArchive::new(file)
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        // Find and read slides
        let mut text = String::new();
        
        for i in 0..archive.len() {
            let file = archive.by_index(i)
                .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
            
            let name = file.name().to_string();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                let mut content = String::new();
                let buf_reader = BufReader::new(file);
                let mut reader = Reader::from_reader(buf_reader);
                reader.trim_text(true);
                
                let mut buf = Vec::new();
                let mut in_text = false;
                
                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Start(ref e)) if e.name().as_ref() == b"a:t" => {
                            in_text = true;
                        }
                        Ok(Event::Text(e)) if in_text => {
                            if let Ok(t) = e.unescape() {
                                content.push_str(&t);
                                content.push(' ');
                            }
                        }
                        Ok(Event::End(ref e)) if e.name().as_ref() == b"a:t" => {
                            in_text = false;
                        }
                        Ok(Event::Eof) => break,
                        Err(e) => return Err(ProcessError::ExtractionFailed(e.to_string())),
                        _ => (),
                    }
                    buf.clear();
                }
                
                if !content.is_empty() {
                    text.push_str(&content);
                    text.push('\n');
                }
            }
        }
        
        let text = clean_text(&text);
        if !text.is_empty() {
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }
} 