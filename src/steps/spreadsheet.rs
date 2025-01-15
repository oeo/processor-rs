use anyhow::Result;
use calamine::{Reader, open_workbook, Xlsx, Range, DataType, XlsxError};
use std::path::Path;
use std::io::BufReader;
use std::fs::File;
use async_trait::async_trait;
use crate::proto::processor::Query;
use crate::types::{ProcessError, Strategy, Config};
use crate::processor::{ProcessingStep, AsyncProcessor, validate_sheet_range};

pub struct SpreadsheetProcessor;

#[async_trait]
impl AsyncProcessor for SpreadsheetProcessor {
    async fn process(&self, query: &mut Query, _config: &Config) -> Result<(), ProcessError> {
        let file_path: &str = query.file_path.as_str();
        if file_path.is_empty() {
            return Err(ProcessError::ExtractionFailed("No file path provided".to_string()));
        }
        
        let mut workbook: Xlsx<BufReader<File>> = open_workbook(Path::new(file_path))
            .map_err(|e: XlsxError| ProcessError::ExtractionFailed(e.to_string()))?;
        
        if let Some(Ok(range)) = workbook.worksheet_range("Sheet1") {
            let text: String = self.process_sheet(&range)?;
            if !text.is_empty() {
                query.prompt_parts.push(text);
            }
        }
        
        Ok(())
    }
}

impl ProcessingStep for SpreadsheetProcessor {
    fn required_for(&self) -> Vec<Strategy> {
        vec![Strategy::Spreadsheet]
    }

    fn name(&self) -> &'static str {
        "spreadsheet_processor"
    }
}

impl SpreadsheetProcessor {
    fn process_sheet(&self, range: &Range<DataType>) -> Result<String, ProcessError> {
        // Get start and end coordinates
        let start: (u32, u32) = range.start().ok_or_else(|| ProcessError::ExtractionFailed("Failed to get range start".to_string()))?;
        let end: (u32, u32) = range.end().ok_or_else(|| ProcessError::ExtractionFailed("Failed to get range end".to_string()))?;
        
        // Validate range
        let (adjusted_start_row, adjusted_start_col, adjusted_end_row, adjusted_end_col) = 
            validate_sheet_range(start.0, start.1, end.0, end.1);

        let mut text = String::new();

        for row in adjusted_start_row..=adjusted_end_row {
            for col in adjusted_start_col..=adjusted_end_col {
                let coords: (u32, u32) = (row, col);
                if let Some(cell) = range.get_value(coords) {
                    match cell {
                        DataType::String(s) => text.push_str(&s),
                        DataType::Float(f) => text.push_str(&f.to_string()),
                        DataType::Int(i) => text.push_str(&i.to_string()),
                        DataType::Bool(b) => text.push_str(&b.to_string()),
                        DataType::DateTime(dt) => text.push_str(&dt.to_string()),
                        _ => ()
                    }
                    text.push(' ');
                }
            }
            text.push('\n');
        }

        Ok(text)
    }
} 