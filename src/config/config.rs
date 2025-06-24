use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use super::enums::LogLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Network
    pub listen_port: u16,
    pub listen_address: String,
    
    // Storage  
    pub storage_dir: PathBuf,
    
    // System
    pub log_level: LogLevel,
    pub daemon_mode: bool,
    
    // Global options from CLI
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Config {
        // Get a proper default directory (cross-platform)
        let default_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nebula");
            
        Config {
            listen_port: 4001,
            listen_address: "0.0.0.0".to_string(),
            storage_dir: default_dir,
            log_level: LogLevel::default(),
            daemon_mode: false,
            verbose: false,
        }
    }
}

impl Config {
    /// Create storage directory if it doesn't exist
    pub fn ensure_storage_dir(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.storage_dir.exists() {
            fs::create_dir_all(&self.storage_dir)?;
        }
        Ok(())
    }
    
    /// Load configuration from a JSON file
    pub fn load_from_file(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to a JSON file
    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        self.ensure_storage_dir()?;
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.listen_port, 4001);
        assert_eq!(config.listen_address, "0.0.0.0");
        assert_eq!(config.log_level, LogLevel::Info);
        assert!(!config.daemon_mode);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        
        // Test serialization to JSON
        let json = serde_json::to_string(&config).expect("Failed to serialize config");
        assert!(json.contains("4001"));
        assert!(json.contains("0.0.0.0"));
        
        // Test deserialization from JSON
        let deserialized: Config = serde_json::from_str(&json).expect("Failed to deserialize config");
        assert_eq!(config.listen_port, deserialized.listen_port);
        assert_eq!(config.listen_address, deserialized.listen_address);
    }

    #[test]
    fn test_config_file_operations() {
        let config = Config::default();
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let temp_path = temp_file.path().to_path_buf();
        
        // Test saving
        config.save_to_file(&temp_path).expect("Failed to save config");
        
        // Test loading
        let loaded_config = Config::load_from_file(&temp_path).expect("Failed to load config");
        assert_eq!(config.listen_port, loaded_config.listen_port);
        assert_eq!(config.listen_address, loaded_config.listen_address);
    }
    
    #[test]
    fn test_storage_directory_creation() {
        let mut config = Config::default();
        // Use a temporary directory for testing
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        config.storage_dir = temp_dir.path().join("test_storage");
        
        // Directory shouldn't exist initially
        assert!(!config.storage_dir.exists());
        
        // Ensure storage directory creation
        config.ensure_storage_dir().expect("Failed to create storage directory");
        
        // Directory should now exist
        assert!(config.storage_dir.exists());
    }
}
