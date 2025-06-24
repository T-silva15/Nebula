use nebula::config::{Config, LogLevel};
use std::path::PathBuf;

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
fn test_log_level_parsing() {
    assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
    assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
    assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
    
    // Test case insensitivity
    assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
    
    // Test invalid input
    assert!("invalid".parse::<LogLevel>().is_err());
}

#[test]
fn test_config_file_operations() {
    use tempfile::NamedTempFile;
    
    let config = Config::default();
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    
    // Test saving
    config.save_to_file(temp_file.path()).expect("Failed to save config");
    
    // Test loading
    let loaded_config = Config::load_from_file(temp_file.path()).expect("Failed to load config");
    assert_eq!(config.listen_port, loaded_config.listen_port);
    assert_eq!(config.listen_address, loaded_config.listen_address);
}
