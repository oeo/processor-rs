fn main() {
    prost_build::compile_protos(&["proto/query.proto"], &["proto/"]).unwrap();
} 