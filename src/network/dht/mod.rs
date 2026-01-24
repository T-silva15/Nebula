//! Distributed Hash Table (DHT) implementation
//!
//! This module provides S/Kademlia-based DHT functionality for content
//! location and routing in the Nebula P2P network.
//!
//! # Features
//! - 160-bit address space with XOR distance metric
//! - k-bucket routing table (k=20)
//! - Iterative FIND_NODE and FIND_VALUE operations
//! - Alpha concurrent requests (α=3)
//! - Content location and provider management
//! - S/Kademlia security extensions

mod config;
mod routing;
mod provider;
mod storage;

pub use config::{DhtConfig, DhtMode};
pub use routing::{RoutingTable, BucketInfo};
pub use provider::{ProviderRecord, ProviderManager};
pub use storage::{DhtStorage, DhtKey, DhtValue};

use libp2p::{
    kad::{self, store::MemoryStore, Behaviour as Kademlia, Config as KademliaConfig, Event as KademliaEvent},
    PeerId,
};
use std::time::Duration;

use crate::content::ContentAddress;
use super::NetworkError;

/// Result type for DHT operations
pub type DhtResult<T> = Result<T, DhtError>;

/// DHT-specific errors
#[derive(Debug, thiserror::Error)]
pub enum DhtError {
    #[error("Key not found in DHT")]
    NotFound,
    
    #[error("DHT query failed: {0}")]
    QueryFailed(String),
    
    #[error("DHT bootstrap failed: {0}")]
    BootstrapFailed(String),
    
    #[error("No providers found for key")]
    NoProviders,
    
    #[error("DHT operation timed out")]
    Timeout,
    
    #[error("DHT not initialized")]
    NotInitialized,
    
    #[error("Invalid key format: {0}")]
    InvalidKey(String),
}

/// Create a Kademlia behaviour for the DHT
pub fn create_kademlia(local_peer_id: PeerId, config: &DhtConfig) -> Kademlia<MemoryStore> {
    let store = MemoryStore::new(local_peer_id);
    
    let mut kad_config = KademliaConfig::default();
    kad_config.set_query_timeout(config.query_timeout);
    kad_config.set_replication_factor(config.replication_factor.try_into().unwrap_or(20.try_into().unwrap()));
    kad_config.set_publication_interval(Some(config.publication_interval));
    kad_config.set_provider_publication_interval(Some(config.provider_publication_interval));
    kad_config.set_provider_record_ttl(Some(config.provider_record_ttl));
    
    let mut kademlia = Kademlia::with_config(local_peer_id, store, kad_config);
    
    // Set mode based on configuration
    match config.mode {
        DhtMode::Server => kademlia.set_mode(Some(kad::Mode::Server)),
        DhtMode::Client => kademlia.set_mode(Some(kad::Mode::Client)),
        DhtMode::Auto => kademlia.set_mode(None),
    }
    
    kademlia
}

/// Convert a ContentAddress to a Kademlia record key
pub fn content_to_kad_key(address: &ContentAddress) -> kad::RecordKey {
    kad::RecordKey::new(&address.to_bytes())
}

/// Convert a Kademlia record key back to a ContentAddress (if possible)
pub fn kad_key_to_content(key: &kad::RecordKey) -> Option<ContentAddress> {
    let bytes = key.as_ref();
    if bytes.len() == 32 {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(bytes);
        Some(ContentAddress::from_hash(hash))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_to_kad_key_roundtrip() {
        let data = b"test content for DHT";
        let address = ContentAddress::from_data(data);
        
        let kad_key = content_to_kad_key(&address);
        let recovered = kad_key_to_content(&kad_key);
        
        assert!(recovered.is_some());
        assert_eq!(recovered.unwrap(), address);
    }
    
    #[test]
    fn test_dht_config_default() {
        let config = DhtConfig::default();
        assert_eq!(config.replication_factor, 20);
        assert_eq!(config.parallelism, 3);
    }
}
