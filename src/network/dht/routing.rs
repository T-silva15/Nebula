//! Kademlia routing table management
//!
//! Provides visibility into and management of the DHT routing table.

#![allow(dead_code)]

use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Information about a routing table bucket
#[derive(Debug, Clone)]
pub struct BucketInfo {
    /// Bucket index (0-159 for 160-bit address space)
    pub index: usize,
    
    /// Number of peers in the bucket
    pub peer_count: usize,
    
    /// Bucket capacity (k value)
    pub capacity: usize,
    
    /// Whether the bucket is full
    pub is_full: bool,
    
    /// Last bucket refresh time
    pub last_refresh: Option<Instant>,
}

/// Information about a peer in the routing table
#[derive(Debug, Clone)]
pub struct PeerRoutingInfo {
    /// Peer ID
    pub peer_id: PeerId,
    
    /// Bucket index this peer belongs to
    pub bucket_index: usize,
    
    /// Last time we successfully communicated with this peer
    pub last_seen: Option<Instant>,
    
    /// RTT estimate for this peer
    pub rtt: Option<Duration>,
    
    /// Number of failed queries to this peer
    pub failed_queries: u32,
    
    /// Whether this peer is considered stale
    pub is_stale: bool,
}

/// Routing table management and analysis
pub struct RoutingTable {
    /// Local peer ID (determines bucket placement for other peers)
    local_peer_id: PeerId,
    
    /// Bucket size (k value)
    bucket_size: usize,
    
    /// Per-peer routing information
    peer_info: HashMap<PeerId, PeerRoutingInfo>,
    
    /// Bucket statistics
    bucket_stats: Vec<BucketInfo>,
    
    /// Last full table refresh
    last_full_refresh: Option<Instant>,
}

impl RoutingTable {
    /// Create a new routing table tracker
    pub fn new(local_peer_id: PeerId, bucket_size: usize) -> Self {
        // Initialize 160 buckets (for 160-bit address space)
        let bucket_stats = (0..160)
            .map(|i| BucketInfo {
                index: i,
                peer_count: 0,
                capacity: bucket_size,
                is_full: false,
                last_refresh: None,
            })
            .collect();
        
        Self {
            local_peer_id,
            bucket_size,
            peer_info: HashMap::new(),
            bucket_stats,
            last_full_refresh: None,
        }
    }
    
    /// Calculate the bucket index for a peer based on XOR distance
    pub fn bucket_for_peer(&self, peer_id: &PeerId) -> usize {
        let local_bytes = self.local_peer_id.to_bytes();
        let peer_bytes = peer_id.to_bytes();
        
        // Find the first differing bit (XOR distance)
        for (i, (a, b)) in local_bytes.iter().zip(peer_bytes.iter()).enumerate() {
            let xor = a ^ b;
            if xor != 0 {
                // Count leading zeros in the XOR result
                let leading_zeros = xor.leading_zeros() as usize;
                return 159 - (i * 8 + leading_zeros);
            }
        }
        
        // Peers are identical (shouldn't happen in practice)
        0
    }
    
    /// Add or update a peer in the routing table
    pub fn update_peer(&mut self, peer_id: PeerId, rtt: Option<Duration>) {
        let bucket_index = self.bucket_for_peer(&peer_id);
        
        let info = self.peer_info.entry(peer_id).or_insert_with(|| PeerRoutingInfo {
            peer_id,
            bucket_index,
            last_seen: None,
            rtt: None,
            failed_queries: 0,
            is_stale: false,
        });
        
        info.last_seen = Some(Instant::now());
        info.rtt = rtt.or(info.rtt);
        info.is_stale = false;
        
        // Update bucket stats
        self.recalculate_bucket_stats();
    }
    
    /// Mark a peer as having failed a query
    pub fn mark_failed(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peer_info.get_mut(peer_id) {
            info.failed_queries += 1;
            
            // Mark as stale after 3 consecutive failures
            if info.failed_queries >= 3 {
                info.is_stale = true;
            }
        }
    }
    
    /// Remove a peer from the routing table
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peer_info.remove(peer_id);
        self.recalculate_bucket_stats();
    }
    
    /// Get peers that are candidates for bucket refresh
    pub fn get_stale_peers(&self) -> Vec<PeerId> {
        self.peer_info
            .values()
            .filter(|info| info.is_stale)
            .map(|info| info.peer_id)
            .collect()
    }
    
    /// Get buckets that need refreshing (empty or not recently refreshed)
    pub fn get_buckets_needing_refresh(&self, max_age: Duration) -> Vec<usize> {
        let now = Instant::now();
        
        self.bucket_stats
            .iter()
            .filter(|bucket| {
                bucket.peer_count == 0 || 
                bucket.last_refresh.map(|t| now.duration_since(t) > max_age).unwrap_or(true)
            })
            .map(|bucket| bucket.index)
            .collect()
    }
    
    /// Get statistics about the routing table
    pub fn stats(&self) -> RoutingTableStats {
        let total_peers = self.peer_info.len();
        let stale_peers = self.peer_info.values().filter(|p| p.is_stale).count();
        let non_empty_buckets = self.bucket_stats.iter().filter(|b| b.peer_count > 0).count();
        let full_buckets = self.bucket_stats.iter().filter(|b| b.is_full).count();
        
        let avg_rtt = if total_peers > 0 {
            let total: Duration = self.peer_info
                .values()
                .filter_map(|p| p.rtt)
                .sum();
            let count = self.peer_info.values().filter(|p| p.rtt.is_some()).count();
            if count > 0 {
                Some(total / count as u32)
            } else {
                None
            }
        } else {
            None
        };
        
        RoutingTableStats {
            total_peers,
            stale_peers,
            non_empty_buckets,
            full_buckets,
            total_buckets: 160,
            bucket_size: self.bucket_size,
            average_rtt: avg_rtt,
        }
    }
    
    /// Get all bucket information
    pub fn bucket_info(&self) -> &[BucketInfo] {
        &self.bucket_stats
    }
    
    fn recalculate_bucket_stats(&mut self) {
        // Reset bucket counts
        for bucket in &mut self.bucket_stats {
            bucket.peer_count = 0;
            bucket.is_full = false;
        }
        
        // Count peers per bucket
        for info in self.peer_info.values() {
            if info.bucket_index < 160 {
                self.bucket_stats[info.bucket_index].peer_count += 1;
            }
        }
        
        // Update full status
        for bucket in &mut self.bucket_stats {
            bucket.is_full = bucket.peer_count >= bucket.capacity;
        }
    }
}

/// Statistics about the routing table
#[derive(Debug, Clone)]
pub struct RoutingTableStats {
    /// Total number of peers in the routing table
    pub total_peers: usize,
    
    /// Number of stale peers
    pub stale_peers: usize,
    
    /// Number of non-empty buckets
    pub non_empty_buckets: usize,
    
    /// Number of full buckets
    pub full_buckets: usize,
    
    /// Total number of buckets (160)
    pub total_buckets: usize,
    
    /// Bucket size (k value)
    pub bucket_size: usize,
    
    /// Average RTT to peers
    pub average_rtt: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    
    #[test]
    fn test_routing_table_creation() {
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());
        
        let table = RoutingTable::new(local_peer_id, 20);
        let stats = table.stats();
        
        assert_eq!(stats.total_peers, 0);
        assert_eq!(stats.total_buckets, 160);
        assert_eq!(stats.bucket_size, 20);
    }
    
    #[test]
    fn test_bucket_calculation() {
        let keypair1 = Keypair::generate_ed25519();
        let keypair2 = Keypair::generate_ed25519();
        
        let local_peer = PeerId::from(keypair1.public());
        let remote_peer = PeerId::from(keypair2.public());
        
        let table = RoutingTable::new(local_peer, 20);
        let bucket = table.bucket_for_peer(&remote_peer);
        
        // Bucket should be between 0 and 159
        assert!(bucket < 160);
    }
    
    #[test]
    fn test_peer_updates() {
        let keypair1 = Keypair::generate_ed25519();
        let keypair2 = Keypair::generate_ed25519();
        
        let local_peer = PeerId::from(keypair1.public());
        let remote_peer = PeerId::from(keypair2.public());
        
        let mut table = RoutingTable::new(local_peer, 20);
        
        // Add peer
        table.update_peer(remote_peer, Some(Duration::from_millis(50)));
        
        let stats = table.stats();
        assert_eq!(stats.total_peers, 1);
        assert_eq!(stats.average_rtt, Some(Duration::from_millis(50)));
        
        // Mark as failed
        table.mark_failed(&remote_peer);
        table.mark_failed(&remote_peer);
        table.mark_failed(&remote_peer);
        
        assert_eq!(table.get_stale_peers().len(), 1);
    }
}
