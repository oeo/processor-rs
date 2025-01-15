use std::path::PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use anyhow::Result;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Utc};

use processor_rs::{
    Config, Strategy, QueryOutput, Processor,
    steps::{TextProcessor, SpreadsheetProcessor, PDFProcessor, OfficeProcessor, ImageProcessor},
    proto::processor::{Query, QueryMetadata},
};

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Output as JSON (default)
    Json,
    /// Output as HTML visualization
    Html,
    /// Output as base64-encoded protobuf
    Protobuf,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a document and return a structured document for LLM digestion 
    Run {
        /// Input file to process
        #[arg(value_name = "FILE")]
        input: PathBuf,
        
        /// Output format (json, html, or protobuf)
        #[arg(long, value_enum, default_value = "json")]
        format: OutputFormat,
        
        /// Custom configuration file (TOML format)
        #[arg(long)]
        config: Option<PathBuf>,
        
        /// Custom temporary directory
        #[arg(long)]
        temp_dir: Option<PathBuf>,
        
        /// Don't delete temporary files
        #[arg(long)]
        keep_temps: bool,
        
        /// Enable detailed logging
        #[arg(long)]
        verbose: bool,
        
        /// Memory limit in megabytes
        #[arg(long)]
        max_memory: Option<u64>,
        
        /// Processing timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
    },
}

fn generate_html(query: &Query) -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Document Processing Results</title>
    <style>
        * {
            font-family: monospace;
            font-size: 13px;
            line-height: 1;
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            font-weight: normal;
            color: #000;
        }
        body {
            background: #fff;
            padding: 40px;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        .section {
            margin: 20px 0;
        }
        hr {
            border: none;
            border-top: 1px solid #ddd;
            margin: 20px 0;
        }
        .metadata {
            display: grid;
            grid-template-columns: 200px 1fr;
            gap: 12px;
        }
        .prompt-part {
            max-height: 400px;
            overflow-y: auto;
            background: #f5f5f5;
            padding: 20px;
            white-space: pre;
            margin: 12px 0;
            line-height: 1.1;
        }
        .attachment {
            margin: 20px 0;
        }
        .attachment img {
            max-width: 100%;
            border: 1px solid #ddd;
        }
        h1, h2, h3 {
            margin: 0 0 16px 0;
        }
        ::-webkit-scrollbar {
            width: 8px;
            height: 8px;
        }
        ::-webkit-scrollbar-track {
            background: #f5f5f5;
        }
        ::-webkit-scrollbar-thumb {
            background: #ddd;
        }
        ::-webkit-scrollbar-thumb:hover {
            background: #ccc;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Document Processing Results</h1>
"#);

    // Basic Information
    html.push_str("<div class='section'>");
    html.push_str("<h2>Basic Information</h2>");
    html.push_str("<div class='metadata'>");
    html.push_str(&format!("<div class='label'>File Type:</div><div class='value'>{}</div>", query.file_type));
    html.push_str(&format!("<div class='label'>File Path:</div><div class='value'>{}</div>", query.file_path));
    html.push_str(&format!("<div class='label'>Strategy:</div><div class='value'>{}</div>", query.strategy));
    html.push_str(&format!("<div class='label'>System Prompt:</div><div class='value'>{}</div>", query.system));
    html.push_str("</div></div>");
    html.push_str("<hr>");

    // Extracted Content
    if !query.prompt_parts.is_empty() {
        html.push_str("<div class='section'>");
        html.push_str("<h2>Extracted Content</h2>");
        for part in &query.prompt_parts {
            html.push_str(&format!("<div class='prompt-part'>{}</div>", part));
        }
        html.push_str("</div>");
        html.push_str("<hr>");
    }

    // Attachments
    if !query.attachments.is_empty() {
        html.push_str("<div class='section'>");
        html.push_str("<h2>Attachments</h2>");
        for att in &query.attachments {
            html.push_str("<div class='attachment'>");
            html.push_str(&format!("<h3>Page {}</h3>", att.page));
            html.push_str(&format!(
                "<img src='data:image/png;base64,{}' alt='Page {}'>",
                BASE64.encode(&att.data),
                att.page
            ));
            html.push_str("</div>");
        }
        html.push_str("</div>");
        html.push_str("<hr>");
    }

    // Metadata
    if let Some(meta) = &query.metadata {
        html.push_str("<div class='section'>");
        html.push_str("<h2>Processing Metadata</h2>");
        html.push_str("<div class='metadata'>");
        
        let started = DateTime::<Utc>::from_timestamp(meta.started_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| meta.started_at.to_string());
        
        let completed = DateTime::<Utc>::from_timestamp(meta.completed_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| meta.completed_at.to_string());
        
        html.push_str(&format!("<div class='label'>Started At:</div><div class='value timestamp'>{}</div>", started));
        html.push_str(&format!("<div class='label'>Completed At:</div><div class='value timestamp'>{}</div>", completed));
        html.push_str(&format!("<div class='label'>Duration:</div><div class='value'>{} ms</div>", meta.total_duration_ms));
        html.push_str(&format!("<div class='label'>File Size:</div><div class='value'>{} bytes</div>", meta.original_file_size));
        
        if !meta.errors.is_empty() {
            html.push_str("<div class='label'>Errors:</div><div class='value'>");
            for error in &meta.errors {
                html.push_str(&format!("<div>{}</div>", error));
            }
            html.push_str("</div>");
        }
        
        if !meta.steps.is_empty() {
            html.push_str("<div class='label'>Processing Steps:</div><div class='value'>");
            for step in &meta.steps {
                html.push_str(&format!(
                    "<div>{} - {} ms ({}MB)</div>",
                    step.name, step.duration_ms, step.memory_mb
                ));
            }
            html.push_str("</div>");
        }
        
        html.push_str("</div></div>");
    }

    html.push_str("</div></body></html>");
    html
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run {
            input,
            format,
            config: config_path,
            temp_dir,
            keep_temps,
            verbose,
            max_memory,
            timeout,
        } => {
            // Setup logging based on verbose flag
            if verbose {
                tracing_subscriber::fmt()
                    .with_writer(std::io::stderr)  // Send logs to stderr
                    .with_span_events(FmtSpan::CLOSE)
                    .with_target(false)  // Don't show target
                    .with_thread_ids(false)  // Don't show thread IDs
                    .with_thread_names(false)  // Don't show thread names
                    .with_file(false)  // Don't show file names
                    .with_line_number(false)  // Don't show line numbers
                    .init();
            }
            
            // Load config
            let mut config = if let Some(path) = config_path {
                let content = std::fs::read_to_string(path)?;
                toml::from_str(&content)?
            } else {
                Config::default()
            };
            
            // Override config values
            if let Some(dir) = temp_dir {
                config.temp_dir = dir;
            }
            if let Some(memory) = max_memory {
                config.max_image_size_mb = memory as u32;
            }
            if let Some(t) = timeout {
                config.timeout_seconds = t as u32;
            }
            config.keep_temps = keep_temps;
            
            // Initialize pipeline
            let mut processor = Processor::new(config);
            
            // Add processors
            processor.add_step(TextProcessor);
            processor.add_step(SpreadsheetProcessor);
            processor.add_step(PDFProcessor);
            processor.add_step(OfficeProcessor);
            processor.add_step(ImageProcessor);
            
            // Get file extension and determine strategy
            let extension = input
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("txt");
            
            let strategy = Strategy::from_extension(extension);
            
            let started_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            
            let mut query = Query {
                file_type: extension.to_string(),
                file_path: input.to_string_lossy().to_string(),
                strategy: strategy.to_string(),
                prompt_parts: Vec::new(),
                attachments: Vec::new(),
                system: "You are a helpful assistant.".to_string(),
                prompt: String::new(),
                metadata: Some(QueryMetadata {
                    started_at,
                    completed_at: 0,
                    total_duration_ms: 0,
                    original_file_size: std::fs::metadata(&input)?.len() as i64,
                    errors: Vec::new(),
                    steps: Vec::new(),
                }),
            };
            
            // Process document
            if verbose {
                info!("Processing document: {}", input.display());
            }
            let output = processor.process(&mut query).await?;
            
            // Generate output based on format
            let output_str = match format {
                OutputFormat::Json => {
                    let query_output: QueryOutput = output.into();
                    serde_json::to_string_pretty(&query_output)?
                },
                OutputFormat::Html => {
                    generate_html(&output)
                },
                OutputFormat::Protobuf => {
                    let mut buf = Vec::new();
                    prost::Message::encode(&output, &mut buf)?;
                    BASE64.encode(buf)
                }
            };
            
            // Print to stdout without any extra formatting
            print!("{}", output_str);
        }
    }
    
    Ok(())
}
