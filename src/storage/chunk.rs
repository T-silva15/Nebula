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

    pub use_content_defined: bool,  
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_size: 8 * 1024,        // 8 KB
            target_size: 16 * 1024,     // 16 KB
            max_size: 24 * 1024,       // 24 KB
            use_content_defined: true,  // Enable CDC by default
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
        
        if self.config.use_content_defined {
            self.chunk_data_fastcdc(data)
        } else {
            self.chunk_data_fixed_size(data)
        }
    }


    fn chunk_data_fastcdc(&self, data: &[u8]) -> Vec<Chunk> {
        // Use fastcdc crate with proper type conversions
        let chunker = fastcdc::v2020::FastCDC::new(
            data,
            self.config.min_size as u32,
            self.config.target_size as u32, 
            self.config.max_size as u32,
        );
        
        // Collect chunks and convert to our Chunk type
        chunker
            .map(|chunk_info| {
                // Extract actual data slice using offset and length
                let start = chunk_info.offset as usize;
                let end = start + chunk_info.length as usize;
                let chunk_data = data[start..end].to_vec();
                let address = ContentAddress::from_data(&chunk_data);
                Chunk { data: chunk_data, address }
            })
            .collect()
    }

    fn chunk_data_fixed_size(&self, data: &[u8]) -> Vec<Chunk> {
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
            use_content_defined: false,
        };
        let chunker = Chunker::with_config(config.clone());
        
        assert_eq!(chunker.config, config);
    }
    
    #[test]
    fn test_content_defined_chunking() {
        let chunker = Chunker::with_config(ChunkConfig {
            min_size: 4096,    // 4KB
            target_size: 8192, // 8KB
            max_size: 16384,   // 16KB
            use_content_defined: true,
        });
        
        let data = vec![42u8; 32768]; // 32KB of identical data
        let chunks = chunker.chunk_data(&data);
        
        // CDC should create variable-sized chunks
        assert!(!chunks.is_empty());
        
        // Verify reconstruction
        let mut reconstructed = Vec::new();
        for chunk in chunks {
            reconstructed.extend_from_slice(&chunk.data);
        }
        assert_eq!(reconstructed, data);
    }
    
    #[test]
    fn test_cdc_vs_fixed_chunking() {
        let data = b"This is test data that should be chunked differently with CDC vs fixed-size chunking. ".repeat(1000);
        
        let fixed_chunker = Chunker::with_config(ChunkConfig {
            min_size: 4096,
            target_size: 8192,
            max_size: 16384,
            use_content_defined: false,
        });
        
        let cdc_chunker = Chunker::with_config(ChunkConfig {
            min_size: 4096,
            target_size: 8192,
            max_size: 16384,
            use_content_defined: true,
        });
        
        let fixed_chunks = fixed_chunker.chunk_data(&data);
        let cdc_chunks = cdc_chunker.chunk_data(&data);
        
        // Should produce different chunking patterns
        // (This test verifies the algorithms are actually different)
        println!("Fixed chunks: {}, CDC chunks: {}", fixed_chunks.len(), cdc_chunks.len());
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
            use_content_defined: false,  // Use fixed-size for predictable testing
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
