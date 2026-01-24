//! DHT storage types and utilities
//!
//! Key-value types for DHT operations.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::content::ContentAddress;

/// A key in the DHT
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DhtKey {
    /// The raw key bytes (typically a content hash)
    pub bytes: Vec<u8>,
    
    /// Key type for disambiguation
    pub key_type: DhtKeyType,
}

/// Type of DHT key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DhtKeyType {
    /// Content address (chunk hash)
    Content,
    
    /// File metadata key
    FileMetadata,
    
    /// Peer information key
    PeerInfo,
    
    /// Custom application key
    Custom,
}

impl DhtKey {
    /// Create a content key from a ContentAddress
    pub fn from_content(address: &ContentAddress) -> Self {
        Self {
            bytes: address.to_bytes(),
            key_type: DhtKeyType::Content,
        }
    }
    
    /// Create a file metadata key
    pub fn file_metadata(file_id: &str) -> Self {
        let mut bytes = b"file:".to_vec();
        bytes.extend_from_slice(file_id.as_bytes());
        Self {
            bytes,
            key_type: DhtKeyType::FileMetadata,
        }
    }
    
    /// Create a peer info key
    pub fn peer_info(peer_id: &str) -> Self {
        let mut bytes = b"peer:".to_vec();
        bytes.extend_from_slice(peer_id.as_bytes());
        Self {
            bytes,
            key_type: DhtKeyType::PeerInfo,
        }
    }
    
    /// Create a custom key
    pub fn custom(namespace: &str, key: &[u8]) -> Self {
        let mut bytes = namespace.as_bytes().to_vec();
        bytes.push(b':');
        bytes.extend_from_slice(key);
        Self {
            bytes,
            key_type: DhtKeyType::Custom,
        }
    }
    
    /// Get the key bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A value stored in the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtValue {
    /// The actual value data
    pub data: Vec<u8>,
    
    /// Value type
    pub value_type: DhtValueType,
    
    /// Unix timestamp when this value was created
    pub created_at: u64,
    
    /// TTL in seconds (0 = no expiry)
    pub ttl_secs: u64,
    
    /// Version number for conflict resolution
    pub version: u64,
    
    /// Publisher's peer ID (as string)
    pub publisher: Option<String>,
}

/// Type of DHT value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DhtValueType {
    /// Raw chunk data
    ChunkData,
    
    /// Chunk metadata (size, hash, etc.)
    ChunkMetadata,
    
    /// File metadata (list of chunks, name, etc.)
    FileMetadata,
    
    /// Peer metadata
    PeerMetadata,
    
    /// Custom value
    Custom,
}

impl DhtValue {
    /// Create a new DHT value
    pub fn new(data: Vec<u8>, value_type: DhtValueType) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            data,
            value_type,
            created_at: now,
            ttl_secs: 0,
            version: 1,
            publisher: None,
        }
    }
    
    /// Create a value with TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl_secs = ttl.as_secs();
        self
    }
    
    /// Set the publisher
    pub fn with_publisher(mut self, peer_id: String) -> Self {
        self.publisher = Some(peer_id);
        self
    }
    
    /// Set the version
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }
    
    /// Check if this value has expired
    pub fn is_expired(&self) -> bool {
        if self.ttl_secs == 0 {
            return false;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now > self.created_at + self.ttl_secs
    }
    
    /// Get the age of this value
    pub fn age(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Duration::from_secs(now.saturating_sub(self.created_at))
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// DHT storage interface for local records
pub struct DhtStorage {
    /// Maximum number of records to store
    max_records: usize,
    
    /// Maximum size per record in bytes
    max_record_size: usize,
    
    /// Current number of stored records
    record_count: usize,
    
    /// Total size of stored records
    total_size: usize,
}

impl DhtStorage {
    /// Create new DHT storage with limits
    pub fn new(max_records: usize, max_record_size: usize) -> Self {
        Self {
            max_records,
            max_record_size,
            record_count: 0,
            total_size: 0,
        }
    }
    
    /// Check if we can store a record of given size
    pub fn can_store(&self, size: usize) -> bool {
        size <= self.max_record_size && self.record_count < self.max_records
    }
    
    /// Track a stored record
    pub fn track_stored(&mut self, size: usize) {
        self.record_count += 1;
        self.total_size += size;
    }
    
    /// Track a removed record
    pub fn track_removed(&mut self, size: usize) {
        self.record_count = self.record_count.saturating_sub(1);
        self.total_size = self.total_size.saturating_sub(size);
    }
    
    /// Get storage statistics
    pub fn stats(&self) -> DhtStorageStats {
        DhtStorageStats {
            record_count: self.record_count,
            total_size: self.total_size,
            max_records: self.max_records,
            max_record_size: self.max_record_size,
            utilization: if self.max_records > 0 {
                self.record_count as f64 / self.max_records as f64
            } else {
                0.0
            },
        }
    }
}

/// Statistics about DHT storage
#[derive(Debug, Clone)]
pub struct DhtStorageStats {
    /// Number of records stored
    pub record_count: usize,
    
    /// Total size of stored data
    pub total_size: usize,
    
    /// Maximum records allowed
    pub max_records: usize,
    
    /// Maximum size per record
    pub max_record_size: usize,
    
    /// Storage utilization (0.0 - 1.0)
    pub utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dht_key_content() {
        let content = ContentAddress::from_data(b"test data");
        let key = DhtKey::from_content(&content);
        
        assert_eq!(key.key_type, DhtKeyType::Content);
        assert_eq!(key.bytes.len(), 32);
    }
    
    #[test]
    fn test_dht_key_custom() {
        let key = DhtKey::custom("myapp", b"mykey");
        
        assert_eq!(key.key_type, DhtKeyType::Custom);
        assert!(key.bytes.starts_with(b"myapp:"));
    }
    
    #[test]
    fn test_dht_value_serialization() {
        let value = DhtValue::new(b"test data".to_vec(), DhtValueType::ChunkData)
            .with_ttl(Duration::from_secs(300))
            .with_publisher("peer123".to_string());
        
        let bytes = value.to_bytes().unwrap();
        let recovered = DhtValue::from_bytes(&bytes).unwrap();
        
        assert_eq!(recovered.data, value.data);
        assert_eq!(recovered.value_type, value.value_type);
        assert_eq!(recovered.publisher, Some("peer123".to_string()));
    }
    
    #[test]
    fn test_dht_value_expiry() {
        let value = DhtValue::new(b"test".to_vec(), DhtValueType::Custom)
            .with_ttl(Duration::from_secs(0)); // 0 = no expiry
        
        assert!(!value.is_expired());
    }
    
    #[test]
    fn test_dht_storage_limits() {
        let mut storage = DhtStorage::new(100, 1024);
        
        assert!(storage.can_store(512));
        assert!(!storage.can_store(2048)); // Too large
        
        storage.track_stored(512);
        assert_eq!(storage.stats().record_count, 1);
    }
}
