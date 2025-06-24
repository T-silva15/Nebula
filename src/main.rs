// Entry point for the Nebula application
mod args;
mod config;
mod node;

use node::Node;
use config::Config;
use clap::Parser;
use args::{NebulaArgs, Commands};

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
    // Handle global options
    println!("Log level: {}", config.log_level);
    if let Some(config_file) = &args.config_file {
        println!("Using config file: {:?}", config_file);
    }
    
    match &args.command {
        Commands::Start { port, storage, address, daemon } => {
            // Handle start command
            println!("Starting node on {}:{} with storage at {:?}", address, port, storage);
            if *daemon {
                println!("Running in daemon mode");
            }
            
            // Create and start the node
            let node = Node::new(
                config.listen_address.clone(),
                config.listen_port,
                config.storage_dir.to_string_lossy().to_string(),
                config.log_level.to_string(),
                config.daemon_mode
            );

            let config_file_path = config.storage_dir.join("config.json");
            config.save_to_file(&config_file_path).expect("Failed to save config");
            println!("Node created with ID: {}", node.id);
            // TODO: Actually start the node
        },
        Commands::Status { storage } => {
            // Handle status command
            println!("Checking status for storage at {:?}", storage);
        },
        Commands::Config { storage, show } => {
            // Handle config command
            if *show {
                println!("Showing config for {:?}", storage);
                println!("Current configuration:");
                println!("{:#?}", config);
            }
        },
        Commands::Stop { storage } => {
            // Handle stop command
            println!("Stopping node with storage at {:?}", storage);
        },
    }
}