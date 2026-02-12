#![forbid(unsafe_code)]

pub mod byte_class;

pub use byte_class::{classify, classify_oracle, ByteClass, CLASS_TABLE};
