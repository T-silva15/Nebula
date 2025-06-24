use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
        Config {
            listen_port: 4001,
            listen_address: "0.0.0.0".to_string(),
            storage_dir: PathBuf::from("~/.nebula"),
            log_level: LogLevel::default(),
            daemon_mode: false,
            verbose: false,
        }
    }
}

impl Config {
    /// Load configuration from a JSON file
    pub fn load_from_file(path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to a JSON file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
