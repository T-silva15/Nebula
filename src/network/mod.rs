//! Network module for P2P communication
//! 
//! This module provides peer-to-peer networking capabilities using libp2p,
//! including peer discovery, connection management, and chunk transfer protocols.

#![allow(dead_code)]

pub mod peer;
pub mod transport;
pub mod discovery;
pub mod protocol;
pub mod behaviour;
pub mod swarm;
pub mod dht;

// Re-export commonly used types
pub use protocol::ChunkRequest;
pub use swarm::{NetworkSwarm, NetworkConfig};

use thiserror::Error;

/// Errors that can occur during network operations
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    
    #[error("Discovery error: {0}")]
    Discovery(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Timeout waiting for response")]
    Timeout,
    
    #[error("Network not started")]
    NotStarted,
}

pub type NetworkResult<T> = Result<T, NetworkError>;
