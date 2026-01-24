//! Provider management for DHT content location
//!
//! Manages content provider records and chunk location.

use libp2p::{PeerId, Multiaddr};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::content::ContentAddress;

/// A record indicating that a peer provides a specific piece of content
#[derive(Debug, Clone)]
pub struct ProviderRecord {
    /// Content being provided
    pub content: ContentAddress,
    
    /// Peer providing this content
    pub provider: PeerId,
    
    /// Addresses where the provider can be reached
    pub addresses: Vec<Multiaddr>,
    
    /// When this record was created
    pub created_at: Instant,
    
    /// When this record was last confirmed
    pub last_confirmed: Instant,
    
    /// TTL for this record
    pub ttl: Duration,
}

impl ProviderRecord {
    /// Create a new provider record
    pub fn new(content: ContentAddress, provider: PeerId, addresses: Vec<Multiaddr>, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            content,
            provider,
            addresses,
            created_at: now,
            last_confirmed: now,
            ttl,
        }
    }
    
    /// Check if this record has expired
    pub fn is_expired(&self) -> bool {
        self.last_confirmed.elapsed() > self.ttl
    }
    
    /// Refresh the record
    pub fn refresh(&mut self) {
        self.last_confirmed = Instant::now();
    }
    
    /// Time until expiration
    pub fn time_until_expiry(&self) -> Duration {
        let elapsed = self.last_confirmed.elapsed();
        if elapsed >= self.ttl {
            Duration::ZERO
        } else {
            self.ttl - elapsed
        }
    }
}

/// Manages provider records for content location
pub struct ProviderManager {
    /// Content -> Set of providers
    providers_for_content: HashMap<ContentAddress, HashSet<PeerId>>,
    
    /// Provider -> Set of content they provide
    content_by_provider: HashMap<PeerId, HashSet<ContentAddress>>,
    
    /// Full provider records
    records: HashMap<(ContentAddress, PeerId), ProviderRecord>,
    
    /// Default TTL for new records
    default_ttl: Duration,
    
    /// Local content we provide
    local_content: HashSet<ContentAddress>,
}

impl ProviderManager {
    /// Create a new provider manager
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            providers_for_content: HashMap::new(),
            content_by_provider: HashMap::new(),
            records: HashMap::new(),
            default_ttl,
            local_content: HashSet::new(),
        }
    }
    
    /// Register that a peer provides content
    pub fn add_provider(
        &mut self, 
        content: ContentAddress, 
        provider: PeerId, 
        addresses: Vec<Multiaddr>
    ) {
        // Update content -> providers mapping
        self.providers_for_content
            .entry(content.clone())
            .or_insert_with(HashSet::new)
            .insert(provider);
        
        // Update provider -> content mapping
        self.content_by_provider
            .entry(provider)
            .or_insert_with(HashSet::new)
            .insert(content.clone());
        
        // Store full record
        let key = (content.clone(), provider);
        if let Some(record) = self.records.get_mut(&key) {
            record.refresh();
            if !addresses.is_empty() {
                record.addresses = addresses;
            }
        } else {
            let record = ProviderRecord::new(content, provider, addresses, self.default_ttl);
            self.records.insert(key, record);
        }
    }
    
    /// Register local content (content we provide)
    pub fn add_local_content(&mut self, content: ContentAddress) {
        self.local_content.insert(content);
    }
    
    /// Remove local content
    pub fn remove_local_content(&mut self, content: &ContentAddress) {
        self.local_content.remove(content);
    }
    
    /// Get all local content we should announce
    pub fn get_local_content(&self) -> impl Iterator<Item = &ContentAddress> {
        self.local_content.iter()
    }
    
    /// Get providers for content
    pub fn get_providers(&self, content: &ContentAddress) -> Vec<&ProviderRecord> {
        if let Some(providers) = self.providers_for_content.get(content) {
            providers
                .iter()
                .filter_map(|peer_id| {
                    let key = (content.clone(), *peer_id);
                    self.records.get(&key)
                })
                .filter(|record| !record.is_expired())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get provider count for content
    pub fn provider_count(&self, content: &ContentAddress) -> usize {
        self.get_providers(content).len()
    }
    
    /// Check if any provider exists for content
    pub fn has_providers(&self, content: &ContentAddress) -> bool {
        self.provider_count(content) > 0
    }
    
    /// Get all content provided by a peer
    pub fn get_content_by_provider(&self, provider: &PeerId) -> Vec<&ContentAddress> {
        self.content_by_provider
            .get(provider)
            .map(|content| content.iter().collect())
            .unwrap_or_default()
    }
    
    /// Remove a provider (e.g., when they disconnect)
    pub fn remove_provider(&mut self, provider: &PeerId) {
        if let Some(content_set) = self.content_by_provider.remove(provider) {
            for content in content_set {
                if let Some(providers) = self.providers_for_content.get_mut(&content) {
                    providers.remove(provider);
                    if providers.is_empty() {
                        self.providers_for_content.remove(&content);
                    }
                }
                self.records.remove(&(content, *provider));
            }
        }
    }
    
    /// Clean up expired records
    pub fn cleanup_expired(&mut self) -> usize {
        let expired: Vec<_> = self.records
            .iter()
            .filter(|(_, record)| record.is_expired())
            .map(|(key, _)| key.clone())
            .collect();
        
        let count = expired.len();
        
        for (content, provider) in expired {
            self.records.remove(&(content.clone(), provider));
            
            if let Some(providers) = self.providers_for_content.get_mut(&content) {
                providers.remove(&provider);
                if providers.is_empty() {
                    self.providers_for_content.remove(&content);
                }
            }
            
            if let Some(content_set) = self.content_by_provider.get_mut(&provider) {
                content_set.remove(&content);
                if content_set.is_empty() {
                    self.content_by_provider.remove(&provider);
                }
            }
        }
        
        count
    }
    
    /// Get statistics about providers
    pub fn stats(&self) -> ProviderStats {
        ProviderStats {
            total_content_tracked: self.providers_for_content.len(),
            total_providers: self.content_by_provider.len(),
            total_records: self.records.len(),
            local_content_count: self.local_content.len(),
            expired_records: self.records.values().filter(|r| r.is_expired()).count(),
        }
    }
}

/// Statistics about provider records
#[derive(Debug, Clone)]
pub struct ProviderStats {
    /// Number of unique content items being tracked
    pub total_content_tracked: usize,
    
    /// Number of unique providers
    pub total_providers: usize,
    
    /// Total number of provider records
    pub total_records: usize,
    
    /// Number of local content items
    pub local_content_count: usize,
    
    /// Number of expired records (pending cleanup)
    pub expired_records: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    
    fn test_peer_id() -> PeerId {
        let keypair = Keypair::generate_ed25519();
        PeerId::from(keypair.public())
    }
    
    fn test_content() -> ContentAddress {
        ContentAddress::from_data(b"test content")
    }
    
    #[test]
    fn test_provider_record() {
        let content = test_content();
        let provider = test_peer_id();
        
        let record = ProviderRecord::new(
            content,
            provider,
            vec![],
            Duration::from_secs(60)
        );
        
        assert!(!record.is_expired());
        assert!(record.time_until_expiry() > Duration::from_secs(50));
    }
    
    #[test]
    fn test_provider_manager_add_and_get() {
        let mut manager = ProviderManager::new(Duration::from_secs(300));
        let content = test_content();
        let provider = test_peer_id();
        
        manager.add_provider(content.clone(), provider, vec![]);
        
        assert!(manager.has_providers(&content));
        assert_eq!(manager.provider_count(&content), 1);
        
        let providers = manager.get_providers(&content);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider, provider);
    }
    
    #[test]
    fn test_provider_manager_multiple_providers() {
        let mut manager = ProviderManager::new(Duration::from_secs(300));
        let content = test_content();
        let provider1 = test_peer_id();
        let provider2 = test_peer_id();
        
        manager.add_provider(content.clone(), provider1, vec![]);
        manager.add_provider(content.clone(), provider2, vec![]);
        
        assert_eq!(manager.provider_count(&content), 2);
    }
    
    #[test]
    fn test_remove_provider() {
        let mut manager = ProviderManager::new(Duration::from_secs(300));
        let content = test_content();
        let provider = test_peer_id();
        
        manager.add_provider(content.clone(), provider, vec![]);
        assert!(manager.has_providers(&content));
        
        manager.remove_provider(&provider);
        assert!(!manager.has_providers(&content));
    }
    
    #[test]
    fn test_local_content() {
        let mut manager = ProviderManager::new(Duration::from_secs(300));
        let content = test_content();
        
        manager.add_local_content(content.clone());
        
        let local: Vec<_> = manager.get_local_content().collect();
        assert_eq!(local.len(), 1);
        assert_eq!(local[0], &content);
    }
}
