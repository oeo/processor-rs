pub mod types;
pub mod processor;
pub mod steps;

// Include generated protobuf code
pub mod proto {
    pub mod processor {
        include!(concat!(env!("OUT_DIR"), "/processor.rs"));
    }
}

// Re-export commonly used types
pub use types::{Config, Strategy, ProcessError, QueryOutput};
pub use processor::Processor;
pub use steps::*; 