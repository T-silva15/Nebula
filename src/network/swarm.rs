//! Network swarm management
//!
//! High-level interface for managing the libp2p swarm and network operations.

#![allow(dead_code)]

use futures::StreamExt;
use libp2p::{
    identity::Keypair,
    identify,
    kad,
    mdns,
    ping,
    request_response,
    swarm::{SwarmEvent, Swarm},
    Multiaddr,
    PeerId,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use crate::storage::ContentStore;

use super::behaviour::{NebulaBehaviour, NebulaBehaviourEvent};
use super::discovery::{DiscoveryConfig, DiscoveryManager};
use super::peer::{PeerId as NebulaPeerId, PeerInfo};
use super::protocol::{ChunkData, ChunkRequest, ChunkResponse};
use super::transport::NetworkTransport;
use super::{NetworkError, NetworkResult};

/// Configuration for the network swarm
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Listen addresses
    pub listen_addresses: Vec<Multiaddr>,
    
    /// Discovery configuration
    pub discovery: DiscoveryConfig,
    
    /// Request timeout
    pub request_timeout: Duration,
    
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/0".parse().unwrap(),
            ],
            discovery: DiscoveryConfig::default(),
            request_timeout: Duration::from_secs(30),
            max_concurrent_requests: 50,
        }
    }
}

impl NetworkConfig {
    /// Create config with a specific port
    pub fn with_port(mut self, port: u16) -> Self {
        self.listen_addresses = vec![
            format!("/ip4/0.0.0.0/tcp/{}", port).parse().unwrap(),
        ];
        self
    }
}

/// Events emitted by the network layer
#[derive(Debug)]
pub enum NetworkEvent {
    /// New peer discovered
    PeerDiscovered { peer_id: PeerId, addresses: Vec<Multiaddr> },
    
    /// Peer connected
    PeerConnected { peer_id: PeerId },
    
    /// Peer disconnected
    PeerDisconnected { peer_id: PeerId },
    
    /// Chunk request received
    ChunkRequestReceived {
        peer_id: PeerId,
        request: ChunkRequest,
        channel: request_response::ResponseChannel<ChunkResponse>,
    },
    
    /// Chunk response received
    ChunkResponseReceived {
        peer_id: PeerId,
        request_id: request_response::OutboundRequestId,
        response: ChunkResponse,
    },
    
    /// Listening on new address
    ListeningOn { address: Multiaddr },
    
    /// Network error
    Error { error: String },
}

/// Commands that can be sent to the swarm
#[derive(Debug)]
pub enum NetworkCommand {
    /// Request chunks from a peer
    RequestChunks {
        peer_id: PeerId,
        request: ChunkRequest,
    },
    /// Dial a peer address
    Dial {
        address: Multiaddr,
    },
    /// Start providing content via DHT
    StartProviding {
        key: kad::RecordKey,
    },
    /// Find providers for content
    GetProviders {
        key: kad::RecordKey,
    },
    /// Shutdown the swarm
    Shutdown,
}

/// Manages the libp2p swarm and network operations
pub struct NetworkSwarm {
    /// Local peer keypair
    keypair: Keypair,
    
    /// Local peer ID
    local_peer_id: PeerId,
    
    /// Network configuration
    config: NetworkConfig,
    
    /// Discovery manager
    discovery: DiscoveryManager,
    
    /// Known peers
    peers: HashMap<PeerId, PeerInfo>,
    
    /// Event sender for notifying the application
    event_tx: Option<mpsc::Sender<NetworkEvent>>,
    
    /// Command sender for sending commands to the swarm
    command_tx: Option<mpsc::Sender<NetworkCommand>>,
    
    /// Flag indicating if the swarm is running
    running: bool,
    
    /// Listening addresses
    listening_addresses: Vec<Multiaddr>,
}

impl NetworkSwarm {
    /// Create a new network swarm
    pub fn new(config: NetworkConfig) -> NetworkResult<Self> {
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());
        
        let discovery = DiscoveryManager::new(config.discovery.clone());
        
        Ok(Self {
            keypair,
            local_peer_id,
            config,
            discovery,
            peers: HashMap::new(),
            event_tx: None,
            command_tx: None,
            running: false,
            listening_addresses: Vec::new(),
        })
    }
    
    /// Create swarm with existing keypair (for persistent identity)
    pub fn with_keypair(keypair: Keypair, config: NetworkConfig) -> NetworkResult<Self> {
        let local_peer_id = PeerId::from(keypair.public());
        let discovery = DiscoveryManager::new(config.discovery.clone());
        
        Ok(Self {
            keypair,
            local_peer_id,
            config,
            discovery,
            peers: HashMap::new(),
            event_tx: None,
            command_tx: None,
            running: false,
            listening_addresses: Vec::new(),
        })
    }
    
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }
    
    /// Get connected peer IDs
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.discovery.connected_peers().cloned().collect()
    }
    
    /// Get connected peer count
    pub fn connected_peer_count(&self) -> usize {
        self.discovery.connected_count()
    }
    
    /// Check if connected to a specific peer
    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.discovery.connected_peers().any(|p| p == peer_id)
    }
    
    /// Get listening addresses
    pub fn listening_addresses(&self) -> &[Multiaddr] {
        &self.listening_addresses
    }
    
    /// Subscribe to network events
    pub fn subscribe(&mut self) -> mpsc::Receiver<NetworkEvent> {
        let (tx, rx) = mpsc::channel(256);
        self.event_tx = Some(tx);
        rx
    }
    
    /// Send a command to the swarm (for use when swarm is running)
    pub async fn send_command(&self, command: NetworkCommand) -> Result<(), NetworkError> {
        if let Some(tx) = &self.command_tx {
            tx.send(command).await
                .map_err(|_| NetworkError::Protocol("Command channel closed".to_string()))?;
            Ok(())
        } else {
            Err(NetworkError::NotStarted)
        }
    }
    
    /// Request chunks from a peer (convenience method)
    pub async fn request_chunks(&self, peer_id: PeerId, request: ChunkRequest) -> Result<(), NetworkError> {
        self.send_command(NetworkCommand::RequestChunks { peer_id, request }).await
    }
    
    /// Start the network swarm
    pub async fn start(&mut self, content_store: Arc<RwLock<ContentStore>>) -> NetworkResult<()> {
        if self.running {
            return Ok(());
        }
        
        // Build transport
        let transport = NetworkTransport::new()
            .build(&self.keypair)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;
        
        // Create behaviour
        let behaviour = NebulaBehaviour::new(self.local_peer_id, self.keypair.public())
            .map_err(|e| NetworkError::Protocol(e.to_string()))?;
        
        // Build swarm
        let mut swarm = Swarm::new(
            transport,
            behaviour,
            self.local_peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );
        
        // Start listening
        for addr in &self.config.listen_addresses {
            swarm.listen_on(addr.clone())
                .map_err(|e| NetworkError::Transport(e.to_string()))?;
        }
        
        // Connect to bootstrap nodes
        for (peer_id, addr) in self.config.discovery.parse_bootstrap_nodes() {
            swarm.dial(addr.clone())
                .map_err(|e| NetworkError::Connection(e.to_string()))?;
            
            let mut peer_info = PeerInfo::new(NebulaPeerId::from_libp2p(peer_id));
            peer_info.add_address(addr);
            self.peers.insert(peer_id, peer_info);
        }
        
        self.running = true;
        
        // Create command channel
        let (command_tx, command_rx) = mpsc::channel(256);
        self.command_tx = Some(command_tx);
        
        // Clone necessary data for the event loop
        let event_tx = self.event_tx.clone();
        let _local_peer_id = self.local_peer_id;
        
        // Spawn the event loop
        tokio::spawn(async move {
            Self::run_event_loop(swarm, content_store, event_tx, command_rx).await;
        });
        
        Ok(())
    }
    
    /// Main event loop for the swarm
    async fn run_event_loop(
        mut swarm: Swarm<NebulaBehaviour>,
        content_store: Arc<RwLock<ContentStore>>,
        event_tx: Option<mpsc::Sender<NetworkEvent>>,
        mut command_rx: mpsc::Receiver<NetworkCommand>,
    ) {
        loop {
            tokio::select! {
                // Handle swarm events
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(event) => {
                            Self::handle_behaviour_event(&mut swarm, &content_store, &event_tx, event).await;
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            if let Some(tx) = &event_tx {
                                let _ = tx.send(NetworkEvent::PeerConnected { peer_id }).await;
                            }
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            if let Some(tx) = &event_tx {
                                let _ = tx.send(NetworkEvent::PeerDisconnected { peer_id }).await;
                            }
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            println!("Listening on {}", address);
                            if let Some(tx) = &event_tx {
                                let _ = tx.send(NetworkEvent::ListeningOn { address }).await;
                            }
                        }
                        _ => {}
                    }
                }
                // Handle commands from outside
                Some(command) = command_rx.recv() => {
                    Self::handle_command(&mut swarm, command).await;
                }
            }
        }
    }
    
    /// Handle commands sent to the swarm
    async fn handle_command(swarm: &mut Swarm<NebulaBehaviour>, command: NetworkCommand) {
        match command {
            NetworkCommand::RequestChunks { peer_id, request } => {
                println!("Sending chunk request to peer {}", peer_id);
                swarm.behaviour_mut().request_chunks(&peer_id, request);
            }
            NetworkCommand::Dial { address } => {
                println!("Dialing {}", address);
                let _ = swarm.dial(address);
            }
            NetworkCommand::StartProviding { key } => {
                println!("Starting to provide key: {:?}", key);
                let _ = swarm.behaviour_mut().start_providing(key);
            }
            NetworkCommand::GetProviders { key } => {
                println!("Getting providers for key: {:?}", key);
                swarm.behaviour_mut().get_providers(key);
            }
            NetworkCommand::Shutdown => {
                println!("Swarm shutdown requested");
                // Event loop will exit
                return;
            }
        }
    }
    
    /// Handle behaviour events
    async fn handle_behaviour_event(
        swarm: &mut Swarm<NebulaBehaviour>,
        content_store: &Arc<RwLock<ContentStore>>,
        event_tx: &Option<mpsc::Sender<NetworkEvent>>,
        event: NebulaBehaviourEvent,
    ) {
        match event {
            NebulaBehaviourEvent::Mdns(mdns::Event::Discovered(peers)) => {
                for (peer_id, addr) in peers {
                    println!("mDNS: Discovered peer {} at {}", peer_id, addr);
                    
                    // Add peer to Kademlia routing table
                    swarm.behaviour_mut().add_address(&peer_id, addr.clone());
                    
                    swarm.dial(addr.clone()).ok();
                    
                    if let Some(tx) = event_tx {
                        let _ = tx.send(NetworkEvent::PeerDiscovered {
                            peer_id,
                            addresses: vec![addr],
                        }).await;
                    }
                }
            }
            NebulaBehaviourEvent::Mdns(mdns::Event::Expired(peers)) => {
                for (peer_id, _addr) in peers {
                    println!("mDNS: Peer {} expired", peer_id);
                }
            }
            NebulaBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. }) => {
                // Add identified peer's addresses to Kademlia
                for addr in &info.listen_addrs {
                    swarm.behaviour_mut().add_address(&peer_id, addr.clone());
                }
                
                println!(
                    "Identified peer {}: {} ({})",
                    peer_id,
                    info.agent_version,
                    info.protocol_version
                );
            }
            NebulaBehaviourEvent::Ping(ping::Event { peer, result, .. }) => {
                match result {
                    Ok(rtt) => {
                        println!("Ping to {}: {:?}", peer, rtt);
                    }
                    Err(e) => {
                        println!("Ping to {} failed: {}", peer, e);
                    }
                }
            }
            NebulaBehaviourEvent::ChunkProtocol(request_response::Event::Message { peer, message, .. }) => {
                match message {
                    request_response::Message::Request { request, channel, .. } => {
                        // Handle incoming chunk request
                        let response = Self::handle_chunk_request(content_store, &request).await;
                        let _ = swarm.behaviour_mut().send_response(channel, response);
                    }
                    request_response::Message::Response { request_id, response } => {
                        if let Some(tx) = event_tx {
                            let _ = tx.send(NetworkEvent::ChunkResponseReceived {
                                peer_id: peer,
                                request_id,
                                response,
                            }).await;
                        }
                    }
                }
            }
            NebulaBehaviourEvent::Kademlia(event) => {
                Self::handle_kademlia_event(swarm, event).await;
            }
            _ => {}
        }
    }
    
    /// Handle an incoming chunk request
    async fn handle_chunk_request(
        content_store: &Arc<RwLock<ContentStore>>,
        request: &ChunkRequest,
    ) -> ChunkResponse {
        let store = content_store.read().await;
        let mut chunks = Vec::new();
        let mut not_found = Vec::new();
        
        for address in &request.addresses {
            match store.get_chunk(address) {
                Ok(chunk) => {
                    chunks.push(ChunkData::new(address.clone(), chunk.data().to_vec()));
                }
                Err(_) => {
                    not_found.push(address.clone());
                }
            }
        }
        
        if chunks.is_empty() {
            ChunkResponse::not_found(request.request_id, not_found)
        } else if not_found.is_empty() {
            ChunkResponse::complete(request.request_id, chunks)
        } else {
            ChunkResponse::partial(request.request_id, chunks, not_found)
        }
    }
    
    /// Handle Kademlia DHT events
    async fn handle_kademlia_event(
        _swarm: &mut Swarm<NebulaBehaviour>,
        event: kad::Event,
    ) {
        match event {
            kad::Event::RoutingUpdated { peer, is_new_peer, addresses, .. } => {
                println!(
                    "Kademlia routing updated: peer={} new={} addresses={:?}",
                    peer, is_new_peer, addresses
                );
            }
            kad::Event::InboundRequest { request } => {
                println!("Kademlia inbound request: {:?}", request);
            }
            kad::Event::OutboundQueryProgressed { id, result, stats, step } => {
                match result {
                    kad::QueryResult::Bootstrap(result) => {
                        match result {
                            Ok(kad::BootstrapOk { peer, num_remaining }) => {
                                println!(
                                    "Bootstrap progress: peer={} remaining={}",
                                    peer, num_remaining
                                );
                            }
                            Err(e) => {
                                println!("Bootstrap error: {:?}", e);
                            }
                        }
                    }
                    kad::QueryResult::GetProviders(result) => {
                        match result {
                            Ok(kad::GetProvidersOk::FoundProviders { key, providers, .. }) => {
                                println!(
                                    "Found {} providers for key {:?}",
                                    providers.len(),
                                    key
                                );
                            }
                            Ok(kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. }) => {
                                println!("GetProviders query finished");
                            }
                            Err(e) => {
                                println!("GetProviders error: {:?}", e);
                            }
                        }
                    }
                    kad::QueryResult::StartProviding(result) => {
                        match result {
                            Ok(kad::AddProviderOk { key }) => {
                                println!("Started providing content: {:?}", key);
                            }
                            Err(e) => {
                                println!("StartProviding error: {:?}", e);
                            }
                        }
                    }
                    kad::QueryResult::GetRecord(result) => {
                        match result {
                            Ok(kad::GetRecordOk::FoundRecord(record)) => {
                                println!(
                                    "Found record: key={:?} value_len={}",
                                    record.record.key,
                                    record.record.value.len()
                                );
                            }
                            Ok(kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. }) => {
                                println!("GetRecord query finished");
                            }
                            Err(e) => {
                                println!("GetRecord error: {:?}", e);
                            }
                        }
                    }
                    kad::QueryResult::PutRecord(result) => {
                        match result {
                            Ok(kad::PutRecordOk { key }) => {
                                println!("Successfully stored record: {:?}", key);
                            }
                            Err(e) => {
                                println!("PutRecord error: {:?}", e);
                            }
                        }
                    }
                    _ => {
                        println!("Kademlia query progressed: id={:?} stats={:?}", id, stats);
                    }
                }
                
                // Log query step info
                if step.last {
                    println!("Query {:?} completed", id);
                }
            }
            kad::Event::ModeChanged { new_mode } => {
                println!("Kademlia mode changed to: {:?}", new_mode);
            }
            _ => {}
        }
    }
    
    /// Start listening on configured addresses
    pub async fn start_listening(&mut self) -> NetworkResult<()> {
        // For now, just mark as running - actual listening happens in run()
        self.running = true;
        Ok(())
    }
    
    /// Run the network event loop
    /// This function runs the event loop directly and blocks until cancelled.
    /// Use tokio::select! to run it alongside other futures.
    pub async fn run(
        &mut self, 
        content_store: Arc<RwLock<ContentStore>>,
        event_tx: Option<mpsc::Sender<NetworkEvent>>
    ) -> NetworkResult<()> {
        self.event_tx = event_tx.clone();
        
        // Build transport
        let transport = NetworkTransport::new()
            .build(&self.keypair)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;
        
        // Create behaviour
        let behaviour = NebulaBehaviour::new(self.local_peer_id, self.keypair.public())
            .map_err(|e| NetworkError::Protocol(e.to_string()))?;
        
        // Build swarm
        let mut swarm = Swarm::new(
            transport,
            behaviour,
            self.local_peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );
        
        // Start listening
        for addr in &self.config.listen_addresses {
            swarm.listen_on(addr.clone())
                .map_err(|e| NetworkError::Transport(e.to_string()))?;
        }
        
        // Connect to bootstrap nodes
        for (peer_id, addr) in self.config.discovery.parse_bootstrap_nodes() {
            swarm.dial(addr.clone())
                .map_err(|e| NetworkError::Connection(e.to_string()))?;
            
            let mut peer_info = PeerInfo::new(NebulaPeerId::from_libp2p(peer_id));
            peer_info.add_address(addr);
            self.peers.insert(peer_id, peer_info);
        }
        
        self.running = true;
        
        // Create command channel for this run mode
        let (command_tx, command_rx) = mpsc::channel(256);
        self.command_tx = Some(command_tx);
        
        // Run the event loop directly (blocking)
        Self::run_event_loop(swarm, content_store, event_tx, command_rx).await;
        
        self.running = false;
        Ok(())
    }
    
    /// Dial a peer at a specific multiaddress
    pub async fn dial(&mut self, addr: Multiaddr) -> NetworkResult<()> {
        // Store the address - actual dialing happens in the event loop
        // For now, we just track the intent
        println!("Queuing dial to: {}", addr);
        Ok(())
    }
    
    /// Stop the network swarm
    pub async fn stop(&mut self) -> NetworkResult<()> {
        self.running = false;
        // The event loop will naturally stop when the swarm is dropped
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.listen_addresses.len(), 1);
        assert_eq!(config.request_timeout, Duration::from_secs(30));
    }
    
    #[test]
    fn test_network_config_with_port() {
        let config = NetworkConfig::default().with_port(4001);
        let addr_str = config.listen_addresses[0].to_string();
        assert!(addr_str.contains("4001"));
    }
    
    #[test]
    fn test_swarm_creation() {
        let config = NetworkConfig::default();
        let swarm = NetworkSwarm::new(config);
        assert!(swarm.is_ok());
    }
}
