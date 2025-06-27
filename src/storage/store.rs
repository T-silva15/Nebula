use std::path::{Path, PathBuf};
use std::fs;
use std::io;

use crate::content::ContentAddress;
use crate::storage::chunk::{Chunk, Chunker, ChunkConfig};

/// Configuration for storage behavior
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageConfig {
    /// Maximum storage size in bytes (None = unlimited)
    pub max_storage_size: Option<u64>,
    
    /// Replication factor - how many copies of data to maintain
    pub replication_factor: u8,
    
    /// Whether to automatically replicate to other nodes
    pub auto_replicate: bool,
    
    /// Primary content store configuration
    pub store_config: ContentStoreConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_storage_size: None,
            replication_factor: 3, // Keep 3 copies by default
            auto_replicate: true,
            store_config: ContentStoreConfig::default(),
        }
    }
}

/// Errors that can occur during content store operations
#[derive(Debug, thiserror::Error)]
pub enum ContentStoreError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Content not found: {address}")]
    ContentNotFound { address: ContentAddress },
    
    #[error("Invalid content address: {0}")]
    InvalidAddress(String),
    
    #[error("Corruption detected: expected {expected}, got {actual}")]
    Corruption { expected: ContentAddress, actual: ContentAddress },
}

pub type Result<T> = std::result::Result<T, ContentStoreError>;

/// Configuration for the content store
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContentStoreConfig {
    /// Root directory for storing content
    pub storage_path: PathBuf,
    /// Configuration for chunking
    pub chunk_config: ChunkConfig,
    /// Whether to verify content integrity on read
    pub verify_on_read: bool,
}

impl Default for ContentStoreConfig {
    fn default() -> Self {
        // Use user's home directory, fallback to current directory if home not available
        let default_path = dirs::home_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join(".nebula")
            .join("store");
            
        Self {
            storage_path: default_path,
            chunk_config: ChunkConfig::default(),
            verify_on_read: true,
        }
    }
}

/// The ContentStore manages content-addressable storage of chunks
pub struct ContentStore {
    config: ContentStoreConfig,
    objects_dir: PathBuf,
    temp_dir: PathBuf,
}

impl ContentStore {
    /// Create a new ContentStore with the given configuration
    pub fn new(config: ContentStoreConfig) -> Result<Self> {
        let objects_dir = config.storage_path.join("objects");
        let temp_dir = config.storage_path.join("temp");
        
        // Create directory structure
        fs::create_dir_all(&objects_dir)?;
        fs::create_dir_all(&temp_dir)?;
        
        Ok(Self {
            config,
            objects_dir,
            temp_dir,
        })
    }
    
    /// Store a chunk in the content store
    /// Returns the content address of the stored chunk
    pub fn put_chunk(&self, data: &[u8]) -> Result<ContentAddress> {
        let chunk = Chunk::new(data.to_vec());
        let address = chunk.address().clone();
        
        // Check if we already have this content
        let final_path = self.chunk_path(&address);
        if final_path.exists() {
            return Ok(address);
        }
        
        // Write to temporary file first, then atomically move
        let temp_path = self.temp_dir.join(format!("tmp_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let final_path = self.chunk_path(&address);
        
        // Ensure parent directory exists
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write data to temp file
        fs::write(&temp_path, data)?;
        
        // Atomically move to final location
        fs::rename(temp_path, final_path)?;
        
        Ok(address)
    }
    
    /// Retrieve a chunk by its content address
    pub fn get_chunk(&self, address: &ContentAddress) -> Result<Chunk> {
        let path = self.chunk_path(address);
        
        if !path.exists() {
            return Err(ContentStoreError::ContentNotFound { 
                address: address.clone() 
            });
        }
        
        let data = fs::read(&path)?;
        let chunk = Chunk::new(data);
        
        // Verify integrity if enabled
        if self.config.verify_on_read {
            let actual_address = chunk.address();
            if actual_address != address {
                return Err(ContentStoreError::Corruption {
                    expected: address.clone(),
                    actual: actual_address.clone(),
                });
            }
        }
        
        Ok(chunk)
    }
    
    /// Check if a chunk exists in the store
    pub fn has_chunk(&self, address: &ContentAddress) -> Result<bool> {
        Ok(self.chunk_path(address).exists())
    }
    
    /// Store a file by chunking it and return a list of chunk addresses
    pub fn put_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<ContentAddress>> {
        let data = fs::read(file_path)?;
        self.put_data(&data)
    }
    
    /// Store arbitrary data by chunking it
    pub fn put_data(&self, data: &[u8]) -> Result<Vec<ContentAddress>> {
        let chunker = Chunker::with_config(self.config.chunk_config.clone());
        let chunks = chunker.chunk_data(data);
        
        let mut addresses = Vec::new();
        for chunk in chunks {
            let address = self.put_chunk(chunk.data())?;
            addresses.push(address);
        }
        
        Ok(addresses)
    }
    
    /// Reconstruct data from a list of chunk addresses
    pub fn get_data(&self, addresses: &[ContentAddress]) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        
        for address in addresses {
            let chunk = self.get_chunk(address)?;
            data.extend_from_slice(chunk.data());
        }
        
        Ok(data)
    }
    
    /// Write reconstructed data to a file
    pub fn get_file<P: AsRef<Path>>(&self, addresses: &[ContentAddress], output_path: P) -> Result<()> {
        let data = self.get_data(addresses)?;
        fs::write(output_path, data)?;
        Ok(())
    }
    
    /// Get storage statistics
    pub fn stats(&self) -> Result<ContentStoreStats> {
        let mut total_chunks = 0;
        let mut total_size = 0;
        
        fn count_files(dir: &Path, total_chunks: &mut usize, total_size: &mut u64) -> io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    count_files(&path, total_chunks, total_size)?;
                } else {
                    *total_chunks += 1;
                    *total_size += entry.metadata()?.len();
                }
            }
            Ok(())
        }
        
        count_files(&self.objects_dir, &mut total_chunks, &mut total_size)?;
        
        Ok(ContentStoreStats {
            total_chunks,
            total_size,
            storage_path: self.config.storage_path.clone(),
        })
    }
    
    /// Remove a chunk from the store (if it exists)
    pub fn remove_chunk(&self, address: &ContentAddress) -> Result<bool> {
        let path = self.chunk_path(address);
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get the file system path for a chunk given its address
    fn chunk_path(&self, address: &ContentAddress) -> PathBuf {
        let hash_str = address.to_string();
        // Use first 2 characters as subdirectory to avoid too many files in one dir
        let subdir = &hash_str[0..2];
        let filename = &hash_str[2..];
        
        self.objects_dir.join(subdir).join(filename)
    }
    
    /// List all stored chunks with detailed information
    pub fn list_content(&self) -> Result<ContentListing> {
        let mut chunks = Vec::new();
        let mut total_chunks = 0;
        let mut total_size = 0;
        
        fn enumerate_chunks(dir: &Path, chunks: &mut Vec<ChunkInfo>, total_chunks: &mut usize, total_size: &mut u64) -> io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    enumerate_chunks(&path, chunks, total_chunks, total_size)?;
                } else {
                    let metadata = entry.metadata()?;
                    *total_chunks += 1;
                    *total_size += metadata.len();
                    
                    // Reconstruct the content address from the file path
                    if let Some(parent) = path.parent() {
                        if let (Some(subdir), Some(filename)) = (parent.file_name(), path.file_name()) {
                            let subdir_str = subdir.to_string_lossy();
                            let filename_str = filename.to_string_lossy();
                            let hash_str = format!("{}{}", subdir_str, filename_str);
                            
                            // Try to parse as content address
                            if let Ok(address) = hash_str.parse::<ContentAddress>() {
                                chunks.push(ChunkInfo {
                                    address,
                                    size: metadata.len(),
                                    created_at: metadata.created().unwrap_or(std::time::UNIX_EPOCH),
                                    file_path: path.clone(),
                                });
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        
        enumerate_chunks(&self.objects_dir, &mut chunks, &mut total_chunks, &mut total_size)?;
        
        // Sort chunks by creation time (newest first)
        chunks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        let stats = ContentStoreStats {
            total_chunks,
            total_size,
            storage_path: self.config.storage_path.clone(),
        };
        
        Ok(ContentListing { chunks, stats })
    }
}

/// Information about a stored chunk
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub address: ContentAddress,
    pub size: u64,
    pub created_at: std::time::SystemTime,
    pub file_path: PathBuf,
}

impl ChunkInfo {
    /// Format the address for display (shortened)
    pub fn short_address(&self) -> String {
        let addr_str = self.address.to_string();
        if addr_str.len() > 16 {
            format!("{}...{}", &addr_str[0..8], &addr_str[addr_str.len()-8..])
        } else {
            addr_str
        }
    }
    
    /// Format the creation time
    pub fn created_time_string(&self) -> String {
        match std::time::SystemTime::now().duration_since(self.created_at) {
            Ok(duration) => {
                let secs = duration.as_secs();
                if secs < 60 {
                    format!("{} seconds ago", secs)
                } else if secs < 3600 {
                    format!("{} minutes ago", secs / 60)
                } else if secs < 86400 {
                    format!("{} hours ago", secs / 3600)
                } else {
                    format!("{} days ago", secs / 86400)
                }
            }
            Err(_) => "just now".to_string()
        }
    }
}

/// Statistics about the content store
#[derive(Debug, Clone)]
pub struct ContentStoreStats {
    pub total_chunks: usize,
    pub total_size: u64,
    pub storage_path: PathBuf,
}

/// Detailed information about stored content
#[derive(Debug, Clone)]
pub struct ContentListing {
    pub chunks: Vec<ChunkInfo>,
    pub stats: ContentStoreStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_store() -> (ContentStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = ContentStoreConfig {
            storage_path: temp_dir.path().to_path_buf(),
            chunk_config: ChunkConfig::default(),
            verify_on_read: true,
        };
        let store = ContentStore::new(config).unwrap();
        (store, temp_dir)
    }
    
    #[test]
    fn test_put_and_get_chunk() {
        let (store, _temp) = create_test_store();
        
        let data = b"Hello, Nebula!";
        let address = store.put_chunk(data).unwrap();
        
        let retrieved_chunk = store.get_chunk(&address).unwrap();
        assert_eq!(retrieved_chunk.data(), data);
    }
    
    #[test]
    fn test_has_chunk() {
        let (store, _temp) = create_test_store();
        
        let data = b"Hello, Nebula!";
        let address = store.put_chunk(data).unwrap();
        
        assert!(store.has_chunk(&address).unwrap());
        
        // Test with non-existent chunk
        let fake_chunk = Chunk::new(b"fake data".to_vec());
        assert!(!store.has_chunk(fake_chunk.address()).unwrap());
    }
    
    #[test]
    fn test_deduplication() {
        let (store, _temp) = create_test_store();
        
        let data = b"Hello, Nebula!";
        let address1 = store.put_chunk(data).unwrap();
        let address2 = store.put_chunk(data).unwrap();
        
        assert_eq!(address1, address2);
        
        let stats = store.stats().unwrap();
        assert_eq!(stats.total_chunks, 1); // Should only have one copy
    }
    
    #[test]
    fn test_put_and_get_data() {
        let (store, _temp) = create_test_store();
        
        let original_data = b"This is a longer piece of data that might be chunked into multiple pieces depending on the chunking configuration.";
        
        let addresses = store.put_data(original_data).unwrap();
        assert!(!addresses.is_empty());
        
        let retrieved_data = store.get_data(&addresses).unwrap();
        assert_eq!(retrieved_data, original_data);
    }
    
    #[test]
    fn test_corruption_detection() {
        let (store, _temp) = create_test_store();
        
        let data = b"Hello, Nebula!";
        let address = store.put_chunk(data).unwrap();
        
        // Manually corrupt the stored file
        let chunk_path = store.chunk_path(&address);
        fs::write(&chunk_path, b"corrupted data").unwrap();
        
        // Should detect corruption on read
        let result = store.get_chunk(&address);
        assert!(matches!(result, Err(ContentStoreError::Corruption { .. })));
    }
    
    #[test]
    fn test_stats() {
        let (store, _temp) = create_test_store();
        
        let stats = store.stats().unwrap();
        assert_eq!(stats.total_chunks, 0);
        
        store.put_chunk(b"chunk1").unwrap();
        store.put_chunk(b"chunk2").unwrap();
        store.put_chunk(b"chunk1").unwrap(); // duplicate
        
        let stats = store.stats().unwrap();
        assert_eq!(stats.total_chunks, 2); // Should be deduplicated
    }
}