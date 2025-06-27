use super::{Config, LogLevel};

impl Config {
    /// Create config by merging CLI arguments with existing config
    pub fn merge_cli_args(mut self, args: &crate::args::NebulaArgs) -> Config {
        // Apply global CLI options
        if let Some(log_level_str) = &args.log_level {
            if let Ok(log_level) = log_level_str.parse::<LogLevel>() {
                self.log_level = log_level;
            }
        }
        
        // If verbose is set, override log level to debug
        if args.verbose {
            self.log_level = LogLevel::Debug;
            self.verbose = true;
        }
        
        // Apply command-specific arguments based on the command
        match &args.command {
            crate::args::Commands::Start { port, storage, address, daemon } => {
                self.listen_port = *port;
                if let Some(storage_path) = storage {
                    self.storage_dir = storage_path.clone();
                }
                self.listen_address = address.clone();
                self.daemon_mode = *daemon;
            },
            // For commands that specify storage directory
            crate::args::Commands::Put { storage, .. } |
            crate::args::Commands::Get { storage, .. } |
            crate::args::Commands::List { storage, .. } |
            crate::args::Commands::ListFiles { storage, .. } |
            crate::args::Commands::Stats { storage } |
            crate::args::Commands::Status { storage } |
            crate::args::Commands::Config { storage, .. } |
            crate::args::Commands::Stop { storage } => {
                if let Some(storage_path) = storage {
                    self.storage_dir = storage_path.clone();
                }
            },
        }
        
        self
    }

    /// Build config with priority: CLI args > config file > defaults
    pub fn build_from_args(args: &crate::args::NebulaArgs) -> Result<Config, Box<dyn std::error::Error>> {
        // 1. Start with defaults
        let mut config = Config::default();
        
        // 2. If config file specified, load and merge it
        if let Some(config_file) = &args.config_file {
            config = Config::load_from_file(config_file)?;
        }
        
        // 3. Merge CLI arguments (highest priority)
        config = config.merge_cli_args(args);
        
        Ok(config)
    }
}
