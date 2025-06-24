pub struct Config {
    // Network
    pub listen_port: u16,
    pub listen_address: String,
    
    // Storage  
    pub storage_dir: PathBuf,
    
    // System
    pub log_level: String,
    pub daemon_mode: bool,
    
    // Global options from CLI
    pub verbose: bool,
}