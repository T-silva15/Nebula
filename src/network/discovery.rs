//! Peer discovery mechanisms
//!
//! Provides mDNS for local network discovery and bootstrap node support.

use libp2p::{
    mdns,
    Multiaddr,
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Configuration for peer discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable mDNS for local network discovery
    pub enable_mdns: bool,
    
    /// mDNS query interval
    #[serde(with = "humantime_serde")]
    pub mdns_interval: Duration,
    
    /// Bootstrap nodes to connect to on startup
    pub bootstrap_nodes: Vec<String>,
    
    /// Maximum number of peers to maintain
    pub max_peers: usize,
    
    /// Minimum number of peers to maintain
    pub min_peers: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            mdns_interval: Duration::from_secs(30),
            bootstrap_nodes: Vec::new(),
            max_peers: 50,
            min_peers: 3,
        }
    }
}

impl DiscoveryConfig {
    /// Create config with bootstrap nodes
    pub fn with_bootstrap_nodes(mut self, nodes: Vec<String>) -> Self {
        self.bootstrap_nodes = nodes;
        self
    }
    
    /// Disable mDNS (useful for non-local networks)
    pub fn without_mdns(mut self) -> Self {
        self.enable_mdns = false;
        self
    }
    
    /// Parse bootstrap nodes into Multiaddr
    pub fn parse_bootstrap_nodes(&self) -> Vec<(PeerId, Multiaddr)> {
        self.bootstrap_nodes
            .iter()
            .filter_map(|addr_str| {
                // Expected format: /ip4/x.x.x.x/tcp/port/p2p/peer_id
                let addr: Multiaddr = addr_str.parse().ok()?;
                
                // Extract peer ID from the multiaddr
                let peer_id = addr.iter().find_map(|proto| {
                    if let libp2p::multiaddr::Protocol::P2p(peer_id) = proto {
                        Some(peer_id)
                    } else {
                        None
                    }
                })?;
                
                // Remove /p2p/... from the address
                let addr_without_peer: Multiaddr = addr
                    .iter()
                    .filter(|proto| !matches!(proto, libp2p::multiaddr::Protocol::P2p(_)))
                    .collect();
                
                Some((peer_id, addr_without_peer))
            })
            .collect()
    }
}

/// Manages peer discovery state
pub struct DiscoveryManager {
    config: DiscoveryConfig,
    discovered_peers: HashSet<PeerId>,
    connected_peers: HashSet<PeerId>,
}

impl DiscoveryManager {
    /// Create a new discovery manager
    pub fn new(config: DiscoveryConfig) -> Self {
        Self {
            config,
            discovered_peers: HashSet::new(),
            connected_peers: HashSet::new(),
        }
    }
    
    /// Check if we need more peers
    pub fn needs_more_peers(&self) -> bool {
        self.connected_peers.len() < self.config.min_peers
    }
    
    /// Check if we have too many peers
    pub fn has_too_many_peers(&self) -> bool {
        self.connected_peers.len() >= self.config.max_peers
    }
    
    /// Add a discovered peer
    pub fn add_discovered(&mut self, peer_id: PeerId) {
        self.discovered_peers.insert(peer_id);
    }
    
    /// Mark a peer as connected
    pub fn mark_connected(&mut self, peer_id: PeerId) {
        self.connected_peers.insert(peer_id);
        self.discovered_peers.remove(&peer_id);
    }
    
    /// Mark a peer as disconnected
    pub fn mark_disconnected(&mut self, peer_id: &PeerId) {
        self.connected_peers.remove(peer_id);
    }
    
    /// Get count of connected peers
    pub fn connected_count(&self) -> usize {
        self.connected_peers.len()
    }
    
    /// Get all connected peer IDs
    pub fn connected_peers(&self) -> impl Iterator<Item = &PeerId> {
        self.connected_peers.iter()
    }
    
    /// Get discovered but not connected peers
    pub fn pending_peers(&self) -> impl Iterator<Item = &PeerId> {
        self.discovered_peers.iter()
    }
}

/// Create mDNS behaviour for local peer discovery
pub fn create_mdns_behaviour() -> Result<mdns::tokio::Behaviour, Box<dyn std::error::Error>> {
    let config = mdns::Config {
        ttl: Duration::from_secs(60 * 5), // 5 minute TTL
        query_interval: Duration::from_secs(30),
        enable_ipv6: false,
    };
    
    Ok(mdns::tokio::Behaviour::new(config, PeerId::random())?)
}

// humantime_serde module for Duration serialization
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
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert!(config.enable_mdns);
        assert_eq!(config.max_peers, 50);
        assert_eq!(config.min_peers, 3);
    }
    
    #[test]
    fn test_discovery_manager() {
        let config = DiscoveryConfig {
            min_peers: 2,
            max_peers: 5,
            ..Default::default()
        };
        
        let mut manager = DiscoveryManager::new(config);
        
        // Initially needs more peers
        assert!(manager.needs_more_peers());
        assert!(!manager.has_too_many_peers());
        
        // Add some connected peers
        let peer1 = PeerId::random();
        let peer2 = PeerId::random();
        manager.mark_connected(peer1);
        manager.mark_connected(peer2);
        
        // Now has minimum peers
        assert!(!manager.needs_more_peers());
        assert_eq!(manager.connected_count(), 2);
    }
    
    #[test]
    fn test_parse_bootstrap_nodes() {
        let config = DiscoveryConfig::default().with_bootstrap_nodes(vec![
            "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".to_string(),
        ]);
        
        let parsed = config.parse_bootstrap_nodes();
        assert_eq!(parsed.len(), 1);
    }
}
