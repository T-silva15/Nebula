// Main config module

pub mod enums;
pub mod config;
pub mod builders;

// Re-export public items for easier access
pub use enums::{LogLevel, NodeState};
pub use config::Config;
// Builder methods are directly implemented on Config struct
