use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "nebula")]
#[command(version = "0.1.0")]
#[command(author = "Tiago Silva")]
#[command(about = "A distributed P2P file sharing system")]
pub struct NebulaArgs {
    /// Path to configuration file
    #[arg(short, long)]
    pub config_file: Option<PathBuf>,
    
    /// Global log level override
    #[arg(long)]
    pub log_level: Option<String>,
    
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
    
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start a Nebula node
    Start {
        /// Port to listen on for P2P connections
        #[arg(short, long, default_value = "4001")]
        port: u16,
        
        /// Directory to store node data (defaults to platform-specific data directory)
        #[arg(short, long)]
        storage: Option<PathBuf>,
        
        /// Bind address for the node
        #[arg(short, long, default_value = "0.0.0.0")]
        address: String,
        
        /// Run as daemon (background process)
        #[arg(short, long)]
        daemon: bool,
        
        /// Bootstrap node addresses (can be specified multiple times)
        #[arg(long)]
        bootstrap: Vec<String>,
    },
    
    /// Store a file in the distributed file system
    Put {
        /// File path to store
        file: PathBuf,
        /// Optional custom storage location
        #[arg(short, long)]
        storage: Option<PathBuf>,
        /// Output format (id, short, json)
        #[arg(long, default_value = "id")]
        format: String,
    },
    
    /// Retrieve a file from the distributed file system
    Get {
        /// File ID to retrieve
        file_id: String,
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
        /// Optional custom storage location
        #[arg(short, long)]
        storage: Option<PathBuf>,
    },
    
    /// List stored content
    List {
        #[arg(short, long)]
        storage: Option<PathBuf>,
        /// Show detailed information
        #[arg(long)]
        verbose: bool,
    },
    
    /// List registered files
    ListFiles {
        #[arg(short, long)]
        storage: Option<PathBuf>,
        /// Show detailed information
        #[arg(long)]
        verbose: bool,
    },
    
    /// Show storage statistics
    Stats {
        #[arg(short, long)]
        storage: Option<PathBuf>,
    },
    
    /// Show node status and information
    Status {
        /// Storage directory to check (defaults to platform-specific data directory)
        #[arg(short, long)]
        storage: Option<PathBuf>,
    },
    
    /// Display or modify configuration
    Config {
        /// Storage directory (defaults to platform-specific data directory)
        #[arg(short, long)]
        storage: Option<PathBuf>,
        
        /// Show current configuration
        #[arg(long)]
        show: bool,
    },
    
    /// Stop a running node
    Stop {
        /// Storage directory of the node to stop (defaults to platform-specific data directory)
        #[arg(short, long)]
        storage: Option<PathBuf>,
    },
    
    /// Peer management commands
    #[command(subcommand)]
    Peer(PeerCommands),
    
    /// Network status and management
    #[command(subcommand)]
    Network(NetworkCommands),
}

/// Peer management subcommands
#[derive(Subcommand, Debug)]
pub enum PeerCommands {
    /// List connected peers
    List,
    
    /// Connect to a peer
    Connect {
        /// Peer multiaddress (e.g., /ip4/192.168.1.1/tcp/4001/p2p/12D3KooW...)
        address: String,
    },
    
    /// Disconnect from a peer
    Disconnect {
        /// Peer ID to disconnect
        peer_id: String,
    },
    
    /// Show peer information
    Info {
        /// Peer ID to show info for
        peer_id: String,
    },
}

/// Network management subcommands
#[derive(Subcommand, Debug)]
pub enum NetworkCommands {
    /// Show network status
    Status,
    
    /// Show local peer ID and addresses
    Id,
    
    /// List discovered peers (including not connected)
    Discover,
}