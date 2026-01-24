//! DHT configuration options
//!
//! Configuration for the Kademlia DHT behaviour.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// DHT operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DhtMode {
    /// Server mode - stores and provides records
    Server,
    /// Client mode - only queries, doesn't store
    Client,
    /// Auto mode - let libp2p decide based on connectivity
    Auto,
}

impl Default for DhtMode {
    fn default() -> Self {
        Self::Auto
    }
}

/// Configuration for the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtConfig {
    /// Operating mode for the DHT
    pub mode: DhtMode,
    
    /// Replication factor (k) - number of peers to store records on
    /// S/Kademlia recommends k=20
    pub replication_factor: usize,
    
    /// Parallelism factor (α) - concurrent requests per lookup
    /// S/Kademlia recommends α=3
    pub parallelism: usize,
    
    /// Query timeout for DHT operations
    #[serde(with = "humantime_serde")]
    pub query_timeout: Duration,
    
    /// How often to republish records
    #[serde(with = "humantime_serde")]
    pub publication_interval: Duration,
    
    /// How often to republish provider records
    #[serde(with = "humantime_serde")]
    pub provider_publication_interval: Duration,
    
    /// TTL for provider records
    #[serde(with = "humantime_serde")]
    pub provider_record_ttl: Duration,
    
    /// Enable S/Kademlia security extensions
    pub enable_security_extensions: bool,
    
    /// Bootstrap nodes for initial DHT population
    pub bootstrap_nodes: Vec<String>,
    
    /// Minimum number of peers before considering DHT ready
    pub min_peers_for_ready: usize,
    
    /// Enable proximity-based routing optimization
    pub proximity_routing: bool,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            mode: DhtMode::Auto,
            replication_factor: 20,  // k=20 as per S/Kademlia
            parallelism: 3,          // α=3 as per S/Kademlia
            query_timeout: Duration::from_secs(60),
            publication_interval: Duration::from_secs(60 * 60), // 1 hour
            provider_publication_interval: Duration::from_secs(60 * 60 * 12), // 12 hours
            provider_record_ttl: Duration::from_secs(60 * 60 * 24), // 24 hours
            enable_security_extensions: true,
            bootstrap_nodes: Vec::new(),
            min_peers_for_ready: 3,
            proximity_routing: true,
        }
    }
}

impl DhtConfig {
    /// Create a configuration optimized for a local/testing environment
    pub fn for_testing() -> Self {
        Self {
            mode: DhtMode::Server,
            replication_factor: 3,
            parallelism: 2,
            query_timeout: Duration::from_secs(10),
            publication_interval: Duration::from_secs(60),
            provider_publication_interval: Duration::from_secs(300),
            provider_record_ttl: Duration::from_secs(600),
            enable_security_extensions: false,
            bootstrap_nodes: Vec::new(),
            min_peers_for_ready: 1,
            proximity_routing: false,
        }
    }
    
    /// Add a bootstrap node
    pub fn with_bootstrap(mut self, addr: impl Into<String>) -> Self {
        self.bootstrap_nodes.push(addr.into());
        self
    }
    
    /// Set DHT mode
    pub fn with_mode(mut self, mode: DhtMode) -> Self {
        self.mode = mode;
        self
    }
}

// Custom serde for Duration
mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;
    
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = DhtConfig::default();
        assert_eq!(config.replication_factor, 20);
        assert_eq!(config.parallelism, 3);
        assert!(config.enable_security_extensions);
    }
    
    #[test]
    fn test_testing_config() {
        let config = DhtConfig::for_testing();
        assert_eq!(config.replication_factor, 3);
        assert!(!config.enable_security_extensions);
    }
    
    #[test]
    fn test_config_builder() {
        let config = DhtConfig::default()
            .with_mode(DhtMode::Server)
            .with_bootstrap("/ip4/127.0.0.1/tcp/4001/p2p/12D3KooW...");
        
        assert_eq!(config.mode, DhtMode::Server);
        assert_eq!(config.bootstrap_nodes.len(), 1);
    }
}
