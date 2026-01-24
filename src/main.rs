// Entry point for the Nebula application
mod args;
mod config;
mod node;
mod content;
mod storage;
mod file;
mod network;

use node::Node;
use config::Config;
use clap::Parser;
use args::{NebulaArgs, Commands, PeerCommands, NetworkCommands, DhtCommands};
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
        Commands::Start { port, storage, address, daemon, bootstrap } => {
            handle_start_command(*port, storage.as_ref(), address, *daemon, bootstrap, config)
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
        Commands::Peer(peer_cmd) => {
            handle_peer_command(peer_cmd, config)
        }
        Commands::Network(net_cmd) => {
            handle_network_command(net_cmd, config)
        }
        Commands::Dht(dht_cmd) => {
            handle_dht_command(dht_cmd, config)
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
    _bootstrap: &[String],
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    // Use tokio runtime for async network operations
    let rt = tokio::runtime::Runtime::new()?;
    
    rt.block_on(async {
        let mut node = Node::new(
            address.to_string(),
            port,
            crate::config::LogLevel::Info,
            daemon
        )?;
        
        if daemon {
            println!("Starting node in daemon mode with P2P networking...");
            node.start_async().await?;
            
            // Run the network in the background
            println!("Node running in daemon mode. Press Ctrl+C to stop.");
            
            // Set up signal handler
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("\nShutting down daemon...");
                    node.stop_async().await?;
                }
            }
        } else {
            println!("Starting node in interactive mode with P2P networking...");
            node.start_async().await?;
            
            // Show network info
            for line in node.network_status() {
                println!("{}", line);
            }
            
            println!("\nNode started. Press Ctrl+C to stop...");
            
            // Spawn the network event loop in the background
            // Note: We need to take ownership for the spawn, so we use a channel to signal shutdown
            let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            
            // Run network event loop until Ctrl+C
            tokio::select! {
                result = node.run_network() => {
                    if let Err(e) = result {
                        println!("Network error: {}", e);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nStopping node...");
                    drop(shutdown_tx); // Signal shutdown
                }
            }
            
            node.stop_async().await?;
        }
        
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    
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

fn handle_peer_command(
    cmd: &PeerCommands,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    
    rt.block_on(async {
        let mut node = Node::new(
            "127.0.0.1".to_string(),
            4001,
            crate::config::LogLevel::Info,
            false
        )?;
        
        // Initialize network for peer operations
        node.start_async().await?;
        
        match cmd {
            PeerCommands::List => {
                println!("Connected Peers:");
                let peers = node.connected_peers();
                if peers.is_empty() {
                    println!("  No peers connected");
                } else {
                    for peer in peers {
                        println!("  - {}", peer);
                    }
                }
            }
            PeerCommands::Connect { address } => {
                println!("Connecting to: {}", address);
                node.connect_peer(address).await?;
                println!("Connection initiated");
            }
            PeerCommands::Disconnect { peer_id } => {
                println!("Disconnecting from peer: {}", peer_id);
                // TODO: Implement peer disconnection
                println!("Peer disconnection not yet implemented");
            }
            PeerCommands::Info { peer_id } => {
                println!("Peer info for: {}", peer_id);
                // TODO: Implement peer info lookup
                println!("Peer info lookup not yet implemented");
            }
        }
        
        node.stop_async().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    
    Ok(())
}

fn handle_network_command(
    cmd: &NetworkCommands,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    
    rt.block_on(async {
        let mut node = Node::new(
            "127.0.0.1".to_string(),
            4001,
            crate::config::LogLevel::Info,
            false
        )?;
        
        // Initialize network
        node.start_async().await?;
        
        match cmd {
            NetworkCommands::Status => {
                for line in node.network_status() {
                    println!("{}", line);
                }
            }
            NetworkCommands::Id => {
                let status = node.network_status();
                // Find and print just the peer ID line
                for line in &status {
                    if line.contains("Local Peer ID") || line.contains("Listening") {
                        println!("{}", line.trim());
                    }
                }
            }
            NetworkCommands::Discover => {
                println!("Discovering peers...");
                // Give mDNS some time to discover
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                
                let peers = node.connected_peers();
                println!("Discovered {} peer(s):", peers.len());
                for peer in peers {
                    println!("  - {}", peer);
                }
            }
        }
        
        node.stop_async().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    
    Ok(())
}

fn handle_dht_command(
    cmd: &DhtCommands,
    _config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    
    rt.block_on(async {
        let mut node = Node::new(
            "127.0.0.1".to_string(),
            4001,
            crate::config::LogLevel::Info,
            false
        )?;
        
        // Initialize network
        node.start_async().await?;
        
        match cmd {
            DhtCommands::Publish { address } => {
                use crate::content::ContentAddress;
                
                // Parse the content address
                let addr = ContentAddress::from_hex(address)
                    .map_err(|e| format!("Invalid content address: {}", e))?;
                
                println!("Publishing content: {}", addr);
                node.announce_content(&addr).await?;
                println!("Content published to DHT");
            }
            DhtCommands::Find { address } => {
                use crate::content::ContentAddress;
                
                // Parse the content address
                let addr = ContentAddress::from_hex(address)
                    .map_err(|e| format!("Invalid content address: {}", e))?;
                
                println!("Finding providers for: {}", addr);
                let providers = node.find_providers(&addr).await?;
                
                if providers.is_empty() {
                    println!("No providers found (this may be a stub - provider discovery requires running network)");
                } else {
                    println!("Found {} provider(s):", providers.len());
                    for provider in providers {
                        println!("  - {}", provider);
                    }
                }
            }
            DhtCommands::Get { key } => {
                println!("Getting DHT value for key: {}", key);
                // TODO: Implement DHT get
                println!("DHT get not yet implemented");
            }
            DhtCommands::Put { key, value } => {
                println!("Putting DHT value: {} = {}", key, value);
                // TODO: Implement DHT put
                println!("DHT put not yet implemented");
            }
            DhtCommands::Stats => {
                println!("DHT Statistics:");
                println!("  Protocol: /nebula/kad/1.0.0");
                println!("  Replication Factor (k): 20");
                println!("  Parallelism (α): 3");
                // TODO: Get actual stats from the DHT
                println!("  (More stats available when running)");
            }
        }
        
        node.stop_async().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    
    Ok(())
}