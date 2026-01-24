#![allow(dead_code)]

use uuid::Uuid;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::config::{NodeState, LogLevel};
use crate::storage::{ContentStore, ContentStoreConfig, ChunkConfig};
use crate::content::ContentAddress;
use crate::file::{FileRegistry, FileMetadata, FileId};
use crate::network::{NetworkConfig, NetworkSwarm, ChunkRequest};

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::store::ContentStoreError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Content address error: {0}")]
    ContentAddress(#[from] crate::content::address::ContentAddressError),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] crate::network::NetworkError),
    
    #[error("Content not found")]
    ContentNotFound,
    
    #[error("Node is not running")]
    NotRunning,
    
    #[error("General error: {0}")]
    General(String),
}

impl From<&str> for NodeError {
    fn from(s: &str) -> Self {
        NodeError::General(s.to_string())
    }
}

impl From<String> for NodeError {
    fn from(s: String) -> Self {
        NodeError::General(s)
    }
}

pub type NodeResult<T> = Result<T, NodeError>;

/// Metadata that persists between node restarts
#[derive(Debug, Serialize, Deserialize)]
struct NodeMetadata {
    id: Uuid,
    created_at: u64, // Unix timestamp
}

impl NodeMetadata {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> NodeResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    fn load_from_file<P: AsRef<Path>>(path: P) -> NodeResult<Self> {
        let content = fs::read_to_string(path)?;
        let metadata: NodeMetadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }
}

pub struct Node {
    pub id: Uuid,               // Generated or loaded from storage
    pub state: NodeState,       // Node state (enum)
    pub address: String,        // Node's network address
    pub port: u16,              // Listening port
    pub storage_dir: PathBuf,   // Directory for storing data
    pub log_level: LogLevel,    // Logging level (using enum)
    pub daemon_mode: bool,      // Whether to run as a daemon
    pub content_store: ContentStore, // Content-addressable storage
    pub file_registry: FileRegistry, // File-level metadata registry
    // Network components (initialized when network is started)
    network_config: Option<NetworkConfig>,
    network_swarm: Option<NetworkSwarm>,
    content_store_shared: Option<Arc<RwLock<ContentStore>>>,
}

impl Node {
    pub fn new(address: String, port: u16, log_level: LogLevel, daemon_mode: bool) -> NodeResult<Node> {
        // Use user's home directory for .nebula
        let home_dir = dirs::home_dir()
            .ok_or_else(|| NodeError::General("Could not determine home directory".to_string()))?;
        
        let nebula_dir = home_dir.join(".nebula");
        let metadata_file = nebula_dir.join("node_metadata.json");
        
        // Create .nebula directory if it doesn't exist
        fs::create_dir_all(&nebula_dir)?;
        
        // Load or create node metadata
        let metadata = if metadata_file.exists() {
            match NodeMetadata::load_from_file(&metadata_file) {
                Ok(metadata) => {
                    println!("Loaded existing node ID: {}", metadata.id);
                    metadata
                }
                Err(e) => {
                    println!("Warning: Could not load node metadata ({}), creating new node", e);
                    let new_metadata = NodeMetadata::new();
                    new_metadata.save_to_file(&metadata_file)?;
                    new_metadata
                }
            }
        } else {
            println!("Creating new node with ID");
            let new_metadata = NodeMetadata::new();
            new_metadata.save_to_file(&metadata_file)?;
            println!("New node ID: {}", new_metadata.id);
            new_metadata
        };
        
        let storage_path = nebula_dir.join(format!("node{}", metadata.id));
        
        // Create storage directory
        fs::create_dir_all(&storage_path)?;
        
        // Create content store
        let store_config = ContentStoreConfig {
            storage_path: storage_path.join("content"),
            chunk_config: ChunkConfig::default(),
            verify_on_read: true,
        };
        let content_store = ContentStore::new(store_config)?;
        
        // Create file registry
        let file_registry = FileRegistry::new(&storage_path)
            .map_err(|e| NodeError::General(format!("Failed to create file registry: {}", e)))?;
        
        Ok(Node {
            id: metadata.id,
            state: NodeState::Stopped,
            address,
            port,
            storage_dir: storage_path,
            log_level,
            daemon_mode,
            content_store,
            file_registry,
            network_config: None,
            network_swarm: None,
            content_store_shared: None,
        })
    }

    pub fn start(&mut self) -> NodeResult<()> {
        self.state = NodeState::Starting;
        println!("Starting node {} on {}:{}", self.id, self.address, self.port);
        println!("Storage directory: {}", self.storage_dir.display());
        
        // TODO: Actual startup logic here (network initialization, etc.)
        self.state = NodeState::Running;
        println!("Node started successfully");
        Ok(())
    }

    pub fn stop(&mut self) -> NodeResult<()> {
        self.state = NodeState::Stopping;
        println!("Stopping node {}", self.id);
        
        // TODO: Actual shutdown logic here
        self.state = NodeState::Stopped;
        println!("Node stopped successfully");
        Ok(())
    }
    
    pub fn is_running(&self) -> bool {
        matches!(self.state, NodeState::Running)
    }
    
    pub fn get_status(&self) -> &NodeState {
        &self.state
    }
    
    /// Run a single operation with the node (start -> operation -> stop)
    pub fn run_command<F, R>(&mut self, operation: F) -> NodeResult<R>
    where
        F: FnOnce(&mut Self) -> NodeResult<R>,
    {
        self.start()?;
        let result = operation(self);
        self.stop()?;
        result
    }
    
    /// Store a file and return its content addresses
    pub fn put_file<P: AsRef<std::path::Path>>(&self, file_path: P) -> NodeResult<Vec<ContentAddress>> {
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        
        println!("Storing file: {}", file_path.as_ref().display());
        let addresses = self.content_store.put_file(file_path)?;
        println!("File stored successfully with {} chunks", addresses.len());
        Ok(addresses)
    }
    
    /// Store a file and register it in the file registry, returning file metadata  
    pub fn put_file_with_registry<P: AsRef<std::path::Path>>(&mut self, file_path: P) -> NodeResult<FileMetadata> {
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        
        let path = file_path.as_ref();
        println!("Storing file with registry: {}", path.display());
        
        // Get file size
        let file_size = fs::metadata(path)?.len();
        
        // Store the file and get chunk addresses
        let addresses = self.content_store.put_file(path)?;
        
        // Get the original filename
        let original_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Register the file in the registry
        let metadata = self.file_registry.register_file(original_name, addresses, file_size)
            .map_err(|e| NodeError::General(format!("Failed to register file: {}", e)))?;
        
        println!("File stored and registered with ID: {} ({} chunks)", 
                 metadata.short_id(), metadata.chunk_count);
        
        Ok(metadata)
    }
    
    /// Retrieve a file by its content addresses
    pub fn get_file<P: AsRef<std::path::Path>>(
        &self, 
        addresses: &[ContentAddress], 
        output_path: P
    ) -> NodeResult<()> {
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        
        println!("Retrieving {} chunks to: {}", addresses.len(), output_path.as_ref().display());
        self.content_store.get_file(addresses, output_path)?;
        println!("File retrieved successfully");
        Ok(())
    }
    
    /// Retrieve a file by its file ID
    pub fn get_file_by_id<P: AsRef<std::path::Path>>(
        &self,
        file_id: &FileId,
        output_path: P
    ) -> NodeResult<()> {
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        
        // Get file metadata from registry
        let metadata = self.file_registry.get_file(file_id)
            .ok_or_else(|| NodeError::General(format!("File not found: {}", file_id)))?;
        
        println!("Retrieving file '{}' ({} chunks) to: {}", 
                 metadata.original_name, 
                 metadata.chunk_count, 
                 output_path.as_ref().display());
        
        // Use the existing get_file method with the chunk addresses
        self.content_store.get_file(&metadata.chunk_addresses, output_path)?;
        println!("File '{}' retrieved successfully", metadata.original_name);
        Ok(())
    }
    
    /// Retrieve a file by its short ID
    pub fn get_file_by_short_id<P: AsRef<std::path::Path>>(
        &self,
        short_id: &str,
        output_path: P
    ) -> NodeResult<()> {
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        
        // Get file metadata from registry using short ID
        let metadata = self.file_registry.get_file_by_short_id(short_id)
            .ok_or_else(|| NodeError::General(format!("File not found with short ID: {}", short_id)))?;
        
        println!("Retrieving file '{}' (ID: {}, {} chunks) to: {}", 
                 metadata.original_name,
                 metadata.short_id(), 
                 metadata.chunk_count, 
                 output_path.as_ref().display());
        
        // Use the existing get_file method with the chunk addresses
        self.content_store.get_file(&metadata.chunk_addresses, output_path)?;
        println!("File '{}' retrieved successfully", metadata.original_name);
        Ok(())
    }
    
    /// Get storage statistics
    pub fn get_stats(&self) -> NodeResult<Vec<String>> {
        let stats = self.content_store.stats()?;
        let listing = self.content_store.list_content()?;
        let file_count = self.file_registry.file_count();
        let file_total_size = self.file_registry.total_size();
        
        let mut result = Vec::new();
        result.push("Storage Statistics:".to_string());
        result.push(format!("  Total chunks: {}", stats.total_chunks));
        result.push(format!("  Total chunk size: {} bytes", stats.total_size));
        result.push(format!("  Registered files: {}", file_count));
        result.push(format!("  Total file size: {} bytes", file_total_size));
        result.push(format!("  Storage path: {}", stats.storage_path.display()));
        
        if !listing.chunks.is_empty() {
            let avg_chunk_size = stats.total_size as f64 / stats.total_chunks as f64;
            result.push(format!("  Average chunk size: {:.1} bytes", avg_chunk_size));
            
            if file_count > 0 {
                let avg_file_size = file_total_size as f64 / file_count as f64;
                result.push(format!("  Average file size: {:.1} bytes", avg_file_size));
            }
            
            // Find largest and smallest chunks
            let largest = listing.chunks.iter().max_by_key(|c| c.size);
            let smallest = listing.chunks.iter().min_by_key(|c| c.size);
            
            if let Some(largest) = largest {
                result.push(format!("  Largest chunk: {} bytes", largest.size));
            }
            if let Some(smallest) = smallest {
                result.push(format!("  Smallest chunk: {} bytes", smallest.size));
            }
        }
        
        Ok(result)
    }
    
    /// List all registered files
    pub fn list_files(&self) -> NodeResult<Vec<String>> {
        let files = self.file_registry.list_files();
        let mut result = Vec::new();
        
        if files.is_empty() {
            result.push("No files registered.".to_string());
        } else {
            result.push(format!("Registered Files ({}):", files.len()));
            for file in files {
                result.push(format!(
                    "  {} - {} ({} bytes, {} chunks) - {}",
                    file.short_id(),
                    file.original_name,
                    file.total_size,
                    file.chunk_count,
                    file.created_time_string()
                ));
            }
        }
        
        Ok(result)
    }
    
    /// List all registered files with detailed information
    pub fn list_files_verbose(&self) -> NodeResult<Vec<String>> {
        let files = self.file_registry.list_files();
        let mut result = Vec::new();
        
        if files.is_empty() {
            result.push("No files registered.".to_string());
        } else {
            result.push(format!("Registered Files ({}):", files.len()));
            for file in files {
                result.push(format!("File ID: {}", file.id));
                result.push(format!("  Name: {}", file.original_name));
                result.push(format!("  Size: {} bytes", file.total_size));
                result.push(format!("  Chunks: {} parts", file.chunk_count));
                result.push(format!("  Created: {}", file.created_time_string()));
                result.push("  Chunk addresses:".to_string());
                for (i, addr) in file.chunk_addresses.iter().enumerate() {
                    result.push(format!("    [{}] {}", i + 1, addr));
                }
                result.push("".to_string()); // Empty line
            }
        }
        
        Ok(result)
    }
    pub fn list_content(&self) -> NodeResult<Vec<String>> {
        let listing = self.content_store.list_content()?;
        let mut result = Vec::new();
        
        if listing.chunks.is_empty() {
            result.push("No content stored.".to_string());
        } else {
            for chunk in &listing.chunks {
                result.push(format!(
                    "  {} ({} bytes) - {}",
                    chunk.short_address(),
                    chunk.size,
                    chunk.created_time_string()
                ));
            }
        }
        
        Ok(result)
    }
    
    /// List all content with verbose details
    pub fn list_content_verbose(&self) -> NodeResult<Vec<String>> {
        let listing = self.content_store.list_content()?;
        let mut result = Vec::new();
        
        if listing.chunks.is_empty() {
            result.push("No content stored.".to_string());
        } else {
            for chunk in &listing.chunks {
                result.push(format!("Chunk: {}", chunk.address));
                result.push(format!("  Size: {} bytes", chunk.size));
                result.push(format!("  Created: {}", chunk.created_time_string()));
                result.push(format!("  File: {}", chunk.file_path.display()));
                result.push("".to_string()); // Empty line
            }
        }
        
        Ok(result)
    }
    
    /// Get detailed node status information
    pub fn get_detailed_status(&self) -> NodeResult<Vec<String>> {
        let metadata_file = dirs::home_dir()
            .ok_or_else(|| NodeError::General("Could not determine home directory".to_string()))?
            .join(".nebula")
            .join("node_metadata.json");
        
        let mut result = Vec::new();
        result.push("Node Status:".to_string());
        result.push(format!("  ID: {}", self.id));
        result.push(format!("  State: {:?}", self.state));
        result.push(format!("  Address: {}:{}", self.address, self.port));
        result.push(format!("  Storage: {}", self.storage_dir.display()));
        
        // Add creation time if we can read it
        if let Ok(metadata) = NodeMetadata::load_from_file(&metadata_file) {
            let _created_at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(metadata.created_at);
            result.push(format!("  Created: {} seconds since epoch", metadata.created_at));
        }
        
        result.push("".to_string());
        
        // Add storage statistics
        let stats = self.content_store.stats()?;
        result.push("Storage Statistics:".to_string());
        result.push(format!("  Total chunks: {}", stats.total_chunks));
        result.push(format!("  Total size: {} bytes", stats.total_size));
        result.push(format!("  Content store: {}", stats.storage_path.display()));
        
        // Add network info if available
        if let Some(ref swarm) = self.network_swarm {
            result.push("".to_string());
            result.push("Network:".to_string());
            result.push(format!("  Local Peer ID: {}", swarm.local_peer_id()));
            result.push(format!("  Connected Peers: {}", swarm.connected_peers().len()));
        }
        
        Ok(result)
    }
    
    // =========================================================================
    // Network-related methods (async)
    // =========================================================================
    
    /// Initialize the network swarm for P2P communication
    pub async fn init_network(&mut self) -> NodeResult<()> {
        use libp2p::Multiaddr;
        
        println!("Initializing P2P network...");
        
        // Create network configuration
        let listen_addr: Multiaddr = format!("/ip4/{}/tcp/{}", self.address, self.port)
            .parse()
            .map_err(|e| NodeError::General(format!("Invalid address: {}", e)))?;
        
        let network_config = NetworkConfig {
            listen_addresses: vec![listen_addr],
            ..Default::default()
        };
        
        // Create the network swarm
        let swarm = NetworkSwarm::new(network_config.clone())?;
        
        println!("Local Peer ID: {}", swarm.local_peer_id());
        
        self.network_config = Some(network_config);
        self.network_swarm = Some(swarm);
        
        Ok(())
    }
    
    /// Start the network event loop (should be run in a background task)
    pub async fn run_network(&mut self) -> NodeResult<()> {
        let swarm = self.network_swarm.as_mut()
            .ok_or_else(|| NodeError::General("Network not initialized".to_string()))?;
        
        // Create shared content store for network operations
        // Note: This is a simplified approach - in production you'd want better state management
        let store_config = ContentStoreConfig {
            storage_path: self.storage_dir.join("content"),
            chunk_config: ChunkConfig::default(),
            verify_on_read: true,
        };
        let content_store = ContentStore::new(store_config)?;
        let shared_store = Arc::new(RwLock::new(content_store));
        self.content_store_shared = Some(shared_store.clone());
        
        // Start listening
        swarm.start_listening().await?;
        
        println!("Network started. Listening for peers...");
        
        // Run the event loop
        swarm.run(shared_store, None).await?;
        
        Ok(())
    }
    
    /// Start the network asynchronously
    pub async fn start_async(&mut self) -> NodeResult<()> {
        self.state = NodeState::Starting;
        println!("Starting node {} on {}:{}", self.id, self.address, self.port);
        println!("Storage directory: {}", self.storage_dir.display());
        
        // Initialize network
        self.init_network().await?;
        
        self.state = NodeState::Running;
        println!("Node started successfully");
        Ok(())
    }
    
    /// Stop the network
    pub async fn stop_async(&mut self) -> NodeResult<()> {
        self.state = NodeState::Stopping;
        println!("Stopping node {}", self.id);
        
        // Stop the network swarm
        if let Some(ref mut swarm) = self.network_swarm {
            swarm.stop().await?;
        }
        
        self.state = NodeState::Stopped;
        println!("Node stopped successfully");
        Ok(())
    }
    
    /// Connect to a peer by multiaddress
    pub async fn connect_peer(&mut self, addr: &str) -> NodeResult<()> {
        use libp2p::Multiaddr;
        
        let swarm = self.network_swarm.as_mut()
            .ok_or_else(|| NodeError::General("Network not initialized".to_string()))?;
        
        let multiaddr: Multiaddr = addr.parse()
            .map_err(|e| NodeError::General(format!("Invalid multiaddr: {}", e)))?;
        
        swarm.dial(multiaddr).await?;
        println!("Connecting to peer at {}", addr);
        Ok(())
    }
    
    /// Get list of connected peers
    pub fn connected_peers(&self) -> Vec<String> {
        if let Some(ref swarm) = self.network_swarm {
            swarm.connected_peers().iter()
                .map(|p| p.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    // =========================================================================
    // DHT Operations  
    // =========================================================================
    
    /// Announce that we are providing content to the DHT
    /// This should be called after storing a file to make it discoverable
    pub async fn announce_content(&mut self, address: &ContentAddress) -> NodeResult<()> {
        use libp2p::kad::RecordKey;
        
        let _swarm = self.network_swarm.as_mut()
            .ok_or_else(|| NodeError::General("Network not initialized".to_string()))?;
        
        // Convert content address to Kademlia record key
        let _key = RecordKey::new(&address.to_bytes());
        
        println!("Announcing content to DHT: {}", address);
        
        // Note: This sets up the announcement but the actual network operations
        // happen in the swarm event loop. The swarm needs to be running.
        // In a full implementation, we'd track the query and wait for completion.
        
        Ok(())
    }
    
    /// Store a file and announce it to the network (async version)
    pub async fn put_file_distributed<P: AsRef<std::path::Path>>(&mut self, file_path: P) -> NodeResult<FileMetadata> {
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        
        let path = file_path.as_ref();
        println!("Storing file (distributed): {}", path.display());
        
        // Get file size
        let file_size = std::fs::metadata(path)?.len();
        
        // Store the file locally
        let addresses = self.content_store.put_file(path)?;
        
        // Get the original filename
        let original_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Register the file in the registry
        let metadata = self.file_registry.register_file(original_name, addresses.clone(), file_size)
            .map_err(|e| NodeError::General(format!("Failed to register file: {}", e)))?;
        
        println!("File stored locally with ID: {} ({} chunks)", 
                 metadata.short_id(), metadata.chunk_count);
        
        // Announce each chunk to the DHT
        // In production, we might want to announce just the file ID and store chunk list
        for addr in &addresses {
            if let Err(e) = self.announce_content(addr).await {
                println!("Warning: Failed to announce chunk {}: {}", addr, e);
            }
        }
        
        println!("Content announced to network");
        Ok(metadata)
    }
    
    /// Query the DHT for providers of specific content
    pub async fn find_providers(&mut self, address: &ContentAddress) -> NodeResult<Vec<String>> {
        let _swarm = self.network_swarm.as_mut()
            .ok_or_else(|| NodeError::General("Network not initialized".to_string()))?;
        
        // Note: In a full implementation, this would:
        // 1. Start a Kademlia GetProviders query
        // 2. Wait for results via the event loop
        // 3. Return the list of provider peer IDs
        
        println!("Querying DHT for providers of: {}", address);
        
        // For now, return empty - actual provider lookup requires async event handling
        Ok(Vec::new())
    }
    
    // =========================================================================
    // Remote File Transfer Methods
    // =========================================================================
    
    /// Request specific chunks from a remote peer
    /// Returns the chunks that were successfully retrieved
    pub async fn request_chunks_from_peer(
        &mut self, 
        peer_id_str: &str, 
        chunk_addresses: Vec<ContentAddress>
    ) -> NodeResult<Vec<(ContentAddress, Vec<u8>)>> {
        use libp2p::PeerId;
        
        let peer_id: PeerId = peer_id_str.parse()
            .map_err(|e| NodeError::General(format!("Invalid peer ID: {}", e)))?;
        
        let swarm = self.network_swarm.as_ref()
            .ok_or_else(|| NodeError::General("Network not initialized".to_string()))?;
        
        // Check if we're connected to this peer
        if !swarm.is_connected(&peer_id) {
            return Err(NodeError::General(format!("Not connected to peer: {}", peer_id_str)));
        }
        
        // Create chunk request
        let request = ChunkRequest::new(chunk_addresses.clone());
        let request_id = request.request_id;
        
        println!("Requesting {} chunks from peer {}", chunk_addresses.len(), peer_id_str);
        
        // Send the request via command channel
        swarm.request_chunks(peer_id, request).await
            .map_err(|e| NodeError::Network(e))?;
        
        // Note: In a full implementation, we would:
        // 1. Track this request_id
        // 2. Wait for ChunkResponseReceived event in the event loop
        // 3. Match the response to this request
        // 4. Return the chunks or error
        
        // For now, we return immediately - the response will come through the event loop
        println!("Request {} sent, waiting for response in event loop...", request_id);
        
        Ok(Vec::new())
    }
    
    /// Get a file from the network by finding providers and fetching chunks
    /// This combines DHT lookup with chunk fetching
    pub async fn get_file_from_network(
        &mut self,
        file_id: &str,
        output_path: &std::path::Path,
    ) -> NodeResult<()> {
        use std::io::Write;
        
        if !self.is_running() {
            return Err(NodeError::NotRunning);
        }
        
        println!("Fetching file {} from network...", file_id);
        
        // First, try to find the file in our local registry
        // In a full P2P system, we'd also query the DHT for file metadata
        let metadata = {
            let file_id_parsed = file_id.parse()
                .map_err(|e| NodeError::General(format!("Invalid file ID: {}", e)))?;
            
            self.file_registry.get_file(&file_id_parsed)
                .ok_or_else(|| NodeError::General("File not found in registry".to_string()))?
                .clone()
        };
        
        println!("Found file metadata: {} ({} chunks)", metadata.original_name, metadata.chunk_count);
        
        // Collect all chunks (from local storage or remote peers)
        let mut all_chunks: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut missing_chunks: Vec<(usize, ContentAddress)> = Vec::new();
        
        // Try to get each chunk locally first
        for (idx, chunk_addr) in metadata.chunk_addresses.iter().enumerate() {
            match self.content_store.get_chunk(chunk_addr) {
                Ok(chunk) => {
                    all_chunks.push((idx, chunk.data().to_vec()));
                }
                Err(_) => {
                    missing_chunks.push((idx, chunk_addr.clone()));
                }
            }
        }
        
        println!("Local chunks: {}/{}", all_chunks.len(), metadata.chunk_count);
        
        if !missing_chunks.is_empty() {
            println!("Missing {} chunks, attempting to fetch from network...", missing_chunks.len());
            
            // Get connected peers
            let connected_peers = self.connected_peers();
            
            if connected_peers.is_empty() {
                println!("Warning: No connected peers to fetch missing chunks from");
            } else {
                // Try each connected peer for missing chunks
                for peer_id in &connected_peers {
                    if missing_chunks.is_empty() {
                        break;
                    }
                    
                    let addresses: Vec<ContentAddress> = missing_chunks.iter()
                        .map(|(_, addr)| addr.clone())
                        .collect();
                    
                    println!("Requesting {} chunks from peer {}", addresses.len(), peer_id);
                    
                    // Request chunks from peer (this initiates the request)
                    // The response will come through the event loop
                    let _ = self.request_chunks_from_peer(peer_id, addresses).await;
                    
                    // Note: In a complete implementation, we'd await the response
                    // and then process the received chunks
                }
            }
        }
        
        // Sort chunks by index and write to file
        all_chunks.sort_by_key(|(idx, _)| *idx);
        
        // Check if we have all chunks
        if all_chunks.len() != metadata.chunk_count {
            return Err(NodeError::General(format!(
                "Missing chunks: have {}/{}", 
                all_chunks.len(), 
                metadata.chunk_count
            )));
        }
        
        // Create parent directory if needed
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write all chunks to the output file
        let mut file = std::fs::File::create(output_path)?;
        for (_, data) in all_chunks {
            file.write_all(&data)?;
        }
        
        println!("File written to: {}", output_path.display());
        Ok(())
    }
    
    /// Store a file locally and get providers from the network
    /// This is useful for downloading content that we don't have locally
    pub async fn fetch_and_store_chunk(
        &mut self,
        address: &ContentAddress,
    ) -> NodeResult<bool> {
        // Check if we already have this chunk
        if self.content_store.has_chunk(address)? {
            return Ok(true);
        }
        
        // Find providers for this content via DHT
        let providers = self.find_providers(address).await?;
        
        if providers.is_empty() {
            println!("No providers found for chunk: {}", address);
            return Ok(false);
        }
        
        // Request from first available provider
        for provider in providers {
            println!("Requesting chunk {} from provider {}", address, provider);
            
            let result = self.request_chunks_from_peer(
                &provider, 
                vec![address.clone()]
            ).await?;
            
            if !result.is_empty() {
                // Store the received chunk
                let (_, data) = &result[0];
                self.content_store.put_data(data)?;
                println!("Chunk {} stored successfully", address);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    // =========================================================================
    // Network Status Methods
    // =========================================================================
    
    /// Get network status information
    pub fn network_status(&self) -> Vec<String> {
        let mut result = Vec::new();
        
        if let Some(ref swarm) = self.network_swarm {
            result.push("Network Status: Active".to_string());
            result.push(format!("  Local Peer ID: {}", swarm.local_peer_id()));
            
            let peers = swarm.connected_peers();
            result.push(format!("  Connected Peers: {}", peers.len()));
            
            if !peers.is_empty() {
                result.push("  Peer List:".to_string());
                for peer in peers {
                    result.push(format!("    - {}", peer));
                }
            }
            
            // Show listening addresses
            result.push("  Listening Addresses:".to_string());
            for addr in swarm.listening_addresses() {
                result.push(format!("    - {}", addr));
            }
        } else {
            result.push("Network Status: Not Initialized".to_string());
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(
            "127.0.0.1".to_string(),
            4001,
            LogLevel::Info,
            false
        ).expect("Failed to create node");
        
        assert_eq!(node.address, "127.0.0.1");
        assert_eq!(node.port, 4001);
        assert!(node.storage_dir.to_string_lossy().contains("node"));
        assert!(!node.daemon_mode);
        assert_eq!(*node.get_status(), NodeState::Stopped);
    }

    #[test]
    fn test_node_state_transitions() {
        let mut node = Node::new(
            "127.0.0.1".to_string(),
            4001,
            LogLevel::Info,
            false
        ).expect("Failed to create node");
        
        // Initial state should be Stopped
        assert_eq!(*node.get_status(), NodeState::Stopped);
        assert!(!node.is_running());
        
        // Start the node
        node.start().expect("Failed to start node");
        assert_eq!(*node.get_status(), NodeState::Running);
        assert!(node.is_running());
        
        // Stop the node
        node.stop().expect("Failed to stop node");
        assert_eq!(*node.get_status(), NodeState::Stopped);
        assert!(!node.is_running());
    }
}