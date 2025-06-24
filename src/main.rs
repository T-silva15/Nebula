mod args;

use clap::Parser;
use args::{NebulaArgs, Commands};

fn main() {
    cli_parsing();
}

fn cli_parsing () {
    let args = NebulaArgs::parse();
    
    // Handle global options
    let log_level = if args.verbose {
        "debug"
    } else {
        args.log_level.as_deref().unwrap_or("info")
    };
    
    println!("Log level: {}", log_level);
    if let Some(config_file) = &args.config_file {
        println!("Using config file: {:?}", config_file);
    }
    
    match args.command {
        Commands::Start { port, storage, address, daemon } => {
            // Handle start command
            println!("Starting node on {}:{} with storage at {:?}", address, port, storage);
            if daemon {
                println!("Running in daemon mode");
            }
        },
        Commands::Status { storage } => {
            // Handle status command
            println!("Checking status for storage at {:?}", storage);
        },
        Commands::Config { storage, show } => {
            // Handle config command
            if show {
                println!("Showing config for {:?}", storage);
            }
        },
        Commands::Stop { storage } => {
            // Handle stop command
            println!("Stopping node with storage at {:?}", storage);
        },
    }
}