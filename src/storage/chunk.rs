use crate::content::ContentAddress;
use std::path::Path;
use std::fs;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    data: Vec<u8>,
    address: ContentAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChunkConfig {
    pub target_size: usize,
    pub min_size: usize,
    pub max_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_size: 256 * 1024,      // 256 KB
            target_size: 512 * 1024,   // 512 KB  
            max_size: 1024 * 1024,     // 1 MB
        }
    }
}


#[derive(Debug, Clone)]
pub struct Chunker {
    config: ChunkConfig,
}

impl Chunker {
    /// Create chunker with default configuration
    pub fn new() -> Self {
        Self::with_config(ChunkConfig::default())
    }
    
    /// Create chunker with custom configuration
    pub fn with_config(config: ChunkConfig) -> Self {
        Self { config }
    }

    pub fn chunk_data(&self, data: &[u8]) -> Vec<Chunk> {
        if data.is_empty() {
            return vec![];
        }
        
        data.chunks(self.config.target_size)
            .map(|chunk_slice| {
                let chunk_data = chunk_slice.to_vec();
                let address = ContentAddress::from_data(&chunk_data);
                Chunk { data: chunk_data, address }
            })
            .collect()
    }

    pub fn chunk_file(&self, file_path: &Path) -> Result<Vec<Chunk>, ChunkerError> {
        let data: Vec<u8> = fs::read(file_path)
            .map_err(ChunkerError::IoError)?;
        Ok(self.chunk_data(&data))
    }
}

impl Chunk {
    /// Create a new chunk from data
    pub fn new(data: Vec<u8>) -> Self {
        let address = ContentAddress::from_data(&data);
        Self { data, address }
    }
    
    /// Get the chunk's data
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    
    /// Get the chunk's address
    pub fn address(&self) -> &ContentAddress {
        &self.address
    }
}

/// Errors that can occur during chunking
#[derive(Debug)]
pub enum ChunkerError {
    IoError(io::Error),
}

impl std::fmt::Display for ChunkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkerError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ChunkerError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_chunk_creation() {
        let data = b"hello world".to_vec();
        let chunk = Chunk { 
            data: data.clone(), 
            address: ContentAddress::from_data(&data) 
        };
        
        assert_eq!(chunk.data, data);
        
        // Address should be deterministic
        let chunk2 = Chunk { 
            data: data.clone(), 
            address: ContentAddress::from_data(&data) 
        };
        assert_eq!(chunk.address, chunk2.address);
    }
    
    #[test]
    fn test_chunker_default_config() {
        let chunker = Chunker::new();
        let config = ChunkConfig::default();
        
        assert_eq!(chunker.config.min_size, config.min_size);
        assert_eq!(chunker.config.target_size, config.target_size);
        assert_eq!(chunker.config.max_size, config.max_size);
    }
    
    #[test]
    fn test_chunker_custom_config() {
        let config = ChunkConfig {
            min_size: 100,
            target_size: 200,
            max_size: 300,
        };
        let chunker = Chunker::with_config(config.clone());
        
        assert_eq!(chunker.config, config);
    }
    
    #[test]
    fn test_chunk_small_data() {
        let chunker = Chunker::new();
        let data = b"small data";
        let chunks = chunker.chunk_data(data);
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data, data);
    }
    
    #[test]
    fn test_chunk_empty_data() {
        let chunker = Chunker::new();
        let chunks = chunker.chunk_data(&[]);
        assert!(chunks.is_empty());
    }
    
    #[test]
    fn test_chunk_large_data() {
        let chunker = Chunker::with_config(ChunkConfig {
            min_size: 10,
            target_size: 50,  // Small for testing
            max_size: 100,
        });
        
        let large_data = vec![42u8; 150]; // 150 bytes
        let chunks = chunker.chunk_data(&large_data);
        
        // Should create 3 chunks: 50 + 50 + 50 bytes
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].data.len(), 50);
        assert_eq!(chunks[1].data.len(), 50);
        assert_eq!(chunks[2].data.len(), 50);
        
        // Verify we can reconstruct original data
        let mut reconstructed = Vec::new();
        for chunk in chunks {
            reconstructed.extend_from_slice(&chunk.data);
        }
        assert_eq!(reconstructed, large_data);
    }
    
    #[test]
    fn test_chunk_file() -> Result<(), Box<dyn std::error::Error>> {
        let chunker = Chunker::new();
        
        // Create a temporary file
        let mut temp_file = NamedTempFile::new()?;
        let test_data = b"This is test file content for chunking";
        temp_file.write_all(test_data)?;
        
        // Chunk the file
        let chunks = chunker.chunk_file(temp_file.path())?;
        
        assert_eq!(chunks.len(), 1); // Small file, single chunk
        assert_eq!(chunks[0].data, test_data);
        
        Ok(())
    }
    
    #[test]
    fn test_chunk_nonexistent_file() {
        let chunker = Chunker::new();
        let result = chunker.chunk_file(Path::new("nonexistent.txt"));
        
        assert!(result.is_err());
        match result {
            Err(ChunkerError::IoError(_)) => (), // Expected
            _ => panic!("Expected IoError"),
        }
    }
    
    #[test]
    fn test_chunks_have_unique_addresses() {
        let chunker = Chunker::new();
        
        let data1 = b"first chunk data";
        let data2 = b"second chunk data";
        
        let chunks1 = chunker.chunk_data(data1);
        let chunks2 = chunker.chunk_data(data2);
        
        assert_ne!(chunks1[0].address, chunks2[0].address);
    }
    
    #[test]
    fn test_identical_data_same_address() {
        let chunker = Chunker::new();
        let data = b"identical data";
        
        let chunks1 = chunker.chunk_data(data);
        let chunks2 = chunker.chunk_data(data);
        
        assert_eq!(chunks1[0].address, chunks2[0].address);
    }
}
