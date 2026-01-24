//! Peer identity and management
//!
//! Provides types for representing and managing peer identities in the network.

#![allow(dead_code)]

use libp2p::PeerId as Libp2pPeerId;
use libp2p::identity::{Keypair, PublicKey};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, Instant};

/// Wrapper around libp2p's PeerId for our domain
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(Libp2pPeerId);

impl PeerId {
    /// Create a new random peer ID
    pub fn random() -> (Self, Keypair) {
        let keypair = Keypair::generate_ed25519();
        let peer_id = Libp2pPeerId::from(keypair.public());
        (Self(peer_id), keypair)
    }
    
    /// Create from an existing libp2p PeerId
    pub fn from_libp2p(peer_id: Libp2pPeerId) -> Self {
        Self(peer_id)
    }
    
    /// Create from a public key
    pub fn from_public_key(key: &PublicKey) -> Self {
        Self(Libp2pPeerId::from(key.clone()))
    }
    
    /// Get the underlying libp2p PeerId
    pub fn to_libp2p(&self) -> Libp2pPeerId {
        self.0
    }
    
    /// Convert to base58 string representation
    pub fn to_base58(&self) -> String {
        self.0.to_base58()
    }
    
    /// Parse from base58 string
    pub fn from_base58(s: &str) -> Result<Self, String> {
        s.parse::<Libp2pPeerId>()
            .map(Self)
            .map_err(|e| format!("Invalid peer ID: {}", e))
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Libp2pPeerId> for PeerId {
    fn from(peer_id: Libp2pPeerId) -> Self {
        Self(peer_id)
    }
}

impl From<PeerId> for Libp2pPeerId {
    fn from(peer_id: PeerId) -> Self {
        peer_id.0
    }
}

/// State of a peer connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerState {
    /// Peer discovered but not connected
    Discovered,
    /// Currently attempting to connect
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection failed or peer disconnected
    Disconnected,
    /// Peer banned due to misbehavior
    Banned,
}

impl Default for PeerState {
    fn default() -> Self {
        PeerState::Discovered
    }
}

/// Information about a known peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Unique identifier for this peer
    pub id: PeerId,
    
    /// Current connection state
    pub state: PeerState,
    
    /// Known multiaddresses for this peer
    pub addresses: Vec<libp2p::Multiaddr>,
    
    /// When we first discovered this peer
    pub discovered_at: Instant,
    
    /// When we last successfully communicated
    pub last_seen: Option<Instant>,
    
    /// Number of successful chunk transfers
    pub successful_transfers: u64,
    
    /// Number of failed transfers
    pub failed_transfers: u64,
    
    /// Average response latency
    pub avg_latency: Option<Duration>,
    
    /// Reputation score (0-100)
    pub reputation: u8,
}

impl PeerInfo {
    /// Create new peer info for a discovered peer
    pub fn new(id: PeerId) -> Self {
        Self {
            id,
            state: PeerState::Discovered,
            addresses: Vec::new(),
            discovered_at: Instant::now(),
            last_seen: None,
            successful_transfers: 0,
            failed_transfers: 0,
            avg_latency: None,
            reputation: 50, // Start with neutral reputation
        }
    }
    
    /// Create peer info with known addresses
    pub fn with_addresses(id: PeerId, addresses: Vec<libp2p::Multiaddr>) -> Self {
        Self {
            addresses,
            ..Self::new(id)
        }
    }
    
    /// Update state and record the time
    pub fn set_state(&mut self, state: PeerState) {
        self.state = state;
        if state == PeerState::Connected {
            self.last_seen = Some(Instant::now());
        }
    }
    
    /// Add a new address for this peer
    pub fn add_address(&mut self, addr: libp2p::Multiaddr) {
        if !self.addresses.contains(&addr) {
            self.addresses.push(addr);
        }
    }
    
    /// Record a successful transfer
    pub fn record_success(&mut self, latency: Duration) {
        self.successful_transfers += 1;
        self.last_seen = Some(Instant::now());
        
        // Update average latency
        match self.avg_latency {
            Some(avg) => {
                let total = self.successful_transfers;
                let new_avg = (avg.as_millis() as u64 * (total - 1) + latency.as_millis() as u64) / total;
                self.avg_latency = Some(Duration::from_millis(new_avg));
            }
            None => {
                self.avg_latency = Some(latency);
            }
        }
        
        // Increase reputation (max 100)
        self.reputation = (self.reputation + 1).min(100);
    }
    
    /// Record a failed transfer
    pub fn record_failure(&mut self) {
        self.failed_transfers += 1;
        
        // Decrease reputation (min 0)
        self.reputation = self.reputation.saturating_sub(5);
    }
    
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.successful_transfers + self.failed_transfers;
        if total == 0 {
            1.0 // No data, assume good
        } else {
            self.successful_transfers as f64 / total as f64
        }
    }
    
    /// Check if peer should be banned based on reputation
    pub fn should_ban(&self) -> bool {
        self.reputation < 10 && self.failed_transfers > 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_peer_id_creation() {
        let (peer_id, _keypair) = PeerId::random();
        let base58 = peer_id.to_base58();
        let parsed = PeerId::from_base58(&base58).unwrap();
        assert_eq!(peer_id, parsed);
    }
    
    #[test]
    fn test_peer_info_reputation() {
        let (peer_id, _) = PeerId::random();
        let mut info = PeerInfo::new(peer_id);
        
        // Initial reputation should be 50
        assert_eq!(info.reputation, 50);
        
        // Record successes
        info.record_success(Duration::from_millis(100));
        info.record_success(Duration::from_millis(200));
        assert_eq!(info.reputation, 52);
        assert_eq!(info.successful_transfers, 2);
        
        // Record failure
        info.record_failure();
        assert_eq!(info.reputation, 47);
        assert_eq!(info.failed_transfers, 1);
    }
}
