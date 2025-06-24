use nebula::config::Config;
use std::path::PathBuf;
use tempfile::{TempDir, NamedTempFile};

/// Test utilities for creating temporary configurations and directories
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub config: Config,
}

impl TestEnvironment {
    /// Create a new test environment with isolated storage
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage_path = temp_dir.path().to_path_buf();
        
        let config = Config {
            listen_port: 0, // Use random available port
            listen_address: "127.0.0.1".to_string(),
            storage_dir: storage_path,
            log_level: nebula::config::LogLevel::Error, // Quiet during tests
            daemon_mode: false,
            verbose: false,
        };
        
        Self { temp_dir, config }
    }
    
    /// Create multiple test environments for multi-node testing
    pub fn create_multiple(count: usize) -> Vec<Self> {
        (0..count).map(|_| Self::new()).collect()
    }
    
    /// Get a path within the test environment
    pub fn path(&self, relative: &str) -> PathBuf {
        self.temp_dir.path().join(relative)
    }
}

/// Create a temporary config file for testing
pub fn create_test_config_file() -> (NamedTempFile, Config) {
    let config = Config::default();
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    config.save_to_file(temp_file.path()).expect("Failed to save test config");
    (temp_file, config)
}
