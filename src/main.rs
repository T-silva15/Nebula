// Entry point for the Nebula application
mod args;
mod config;
mod node;
mod content;
mod storage;
mod file;

use node::Node;
use config::Config;
use clap::Parser;
use args::{NebulaArgs, Commands};
use uuid;

fn main() {
    let args = NebulaArgs::parse();
    
    // Build configuration from CLI args (includes file loading and merging)
    let config = match Config::build_from_args(&args) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error building configuration: {}", e);
            std::process::exit(1);
        }
    };
    // Handle command line interface commands and options
    handle_cli(&args, &config);

}

fn handle_cli(args: &NebulaArgs, config: &Config) {
    let result = match &args.command {
        Commands::Start { port, storage, address, daemon } => {
            handle_start_command(*port, storage.as_ref(), address, *daemon, config)
        }
        Commands::Put { file, storage, format } => {
            handle_put_command(file, storage.as_ref(), format, config)
        }
        Commands::Get { file_id, output, storage } => {
            handle_get_command(file_id, output, storage.as_ref(), config)
        }
        Commands::List { storage, verbose } => {
            handle_list_command(storage.as_ref(), *verbose, config)
        }
        Commands::ListFiles { storage, verbose } => {
            handle_list_files_command(storage.as_ref(), *verbose, config)
        }
        Commands::Stats { storage } => {
            handle_stats_command(storage.as_ref(), config)
        }
        Commands::Status { storage } => {
            handle_status_command(storage.as_ref(), config)
        }
        Commands::Config { storage, show } => {
            handle_config_command(storage.as_ref(), *show, config)
        }
        Commands::Stop { storage } => {
            handle_stop_command(storage.as_ref(), config)
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn handle_start_command(
    port: u16, 
    _storage: Option<&std::path::PathBuf>, 
    address: &str, 
    daemon: bool,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        address.to_string(),
        port,
        crate::config::LogLevel::Info,
        daemon
    )?;
    
    if daemon {
        println!("Starting node in daemon mode...");
        node.start()?;
        
        // In a real implementation, this would run indefinitely
        // For now, we'll simulate daemon mode
        println!("Node running in daemon mode. Press Ctrl+C to stop.");
        
        // Set up signal handler for graceful shutdown
        match std::thread::park_timeout(std::time::Duration::from_secs(3600)) {
            () => {
                println!("Shutting down daemon...");
                node.stop()?;
            }
        }
    } else {
        println!("Starting node in interactive mode...");
        node.start()?;
        
        println!("Node started. Press Enter to stop...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        node.stop()?;
    }
    
    Ok(())
}

fn handle_put_command(
    file: &std::path::PathBuf,
    _storage: Option<&std::path::PathBuf>,
    format: &str,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        crate::config::LogLevel::Info,
        false
    )?;
    
    node.run_command(|node| {
        let metadata = node.put_file_with_registry(file)?;
        
        match format {
            "id" => {
                println!("{}", metadata.id);
            }
            "short" => {
                println!("{}", metadata.short_id());
            }
            "json" => {
                println!("{}", serde_json::to_string_pretty(&metadata)?);
            }
            "addresses" => {
                // Legacy support for addresses format
                for addr in &metadata.chunk_addresses {
                    println!("{}", addr);
                }
            }
            _ => {
                eprintln!("Unknown format: {}. Supported: id, short, json, addresses", format);
                return Err("Invalid format".into());
            }
        }
        
        Ok(())
    })?;
    
    Ok(())
}

fn handle_get_command(
    file_id: &str,
    output: &std::path::PathBuf,
    _storage: Option<&std::path::PathBuf>,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        crate::config::LogLevel::Info,
        false
    )?;
    
    node.run_command(|node| {
        // Try to parse as full UUID first (file ID)
        if let Ok(parsed_id) = uuid::Uuid::parse_str(file_id) {
            node.get_file_by_id(&parsed_id, output)?;
            println!("File retrieved to: {}", output.display());
        } else if file_id.len() == 8 && file_id.chars().all(|c| c.is_ascii_hexdigit()) {
            // Try as short ID (8 hex characters)
            node.get_file_by_short_id(file_id, output)?;
            println!("File retrieved to: {}", output.display());
        } else {
            // Fall back to treating it as a content address (legacy support)
            let parsed_address = crate::content::ContentAddress::from_hex(file_id)
                .map_err(|e| format!("Invalid file ID, short ID, or content address format: {}", e))?;
            
            println!("Retrieving chunk: {} (legacy mode)", parsed_address);
            let addresses = vec![parsed_address];
            node.get_file(&addresses, output)?;
            println!("Content retrieved to: {}", output.display());
        }
        
        Ok(())
    })?;
    
    Ok(())
}

fn handle_list_command(
    _storage: Option<&std::path::PathBuf>,
    verbose: bool,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        crate::config::LogLevel::Info,
        false
    )?;
    
    node.run_command(|node| {
        if verbose {
            let content = node.list_content_verbose()?;
            println!("Stored Content (detailed):");
            for item in content {
                println!("{}", item);
            }
        } else {
            let content = node.list_content()?;
            println!("Stored Content:");
            for item in content {
                println!("{}", item);
            }
        }
        
        Ok(())
    })?;
    
    Ok(())
}

fn handle_stats_command(
    _storage: Option<&std::path::PathBuf>,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        crate::config::LogLevel::Info,
        false
    )?;
    
    node.run_command(|node| {
        let stats = node.get_stats()?;
        for line in stats {
            println!("{}", line);
        }
        
        Ok(())
    })?;
    
    Ok(())
}

fn handle_status_command(
    _storage: Option<&std::path::PathBuf>,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        crate::config::LogLevel::Info,
        false
    )?;
    
    node.run_command(|node| {
        let status = node.get_detailed_status()?;
        for line in status {
            println!("{}", line);
        }
        Ok(())
    })?;
    
    Ok(())
}

fn handle_config_command(
    _storage: Option<&std::path::PathBuf>,
    show: bool,
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    if show {
        println!("Current configuration:");
        println!("{:#?}", config);
    } else {
        println!("Config modification not yet implemented");
    }
    Ok(())
}

fn handle_stop_command(
    _storage: Option<&std::path::PathBuf>,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Node stop command not yet implemented");
    // TODO: Implement stopping running daemon nodes
    Ok(())
}

fn handle_list_files_command(
    _storage: Option<&std::path::PathBuf>,
    verbose: bool,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        crate::config::LogLevel::Info,
        false
    )?;
    
    node.run_command(|node| {
        if verbose {
            let files = node.list_files_verbose()?;
            println!("Registered Files (detailed):");
            for line in files {
                println!("{}", line);
            }
        } else {
            let files = node.list_files()?;
            for line in files {
                println!("{}", line);
            }
        }
        
        Ok(())
    })?;
    
    Ok(())
}