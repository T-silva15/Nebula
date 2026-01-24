//! Chunk transfer protocol
//!
//! Custom protocol for requesting and transferring content chunks between peers.

use futures::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use libp2p::{
    request_response::{self, Codec, ProtocolSupport},
    StreamProtocol,
};
use serde::{Deserialize, Serialize};
use std::io;

use crate::content::ContentAddress;

/// Protocol identifier for chunk transfers
pub const CHUNK_PROTOCOL: &str = "/nebula/chunk/1.0.0";

/// Request to fetch chunks from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRequest {
    /// Request ID for tracking
    pub request_id: u64,
    
    /// Content addresses of chunks to retrieve
    pub addresses: Vec<ContentAddress>,
    
    /// Maximum total size in bytes (optional limit)
    pub max_size: Option<u64>,
}

impl ChunkRequest {
    /// Create a new chunk request
    pub fn new(addresses: Vec<ContentAddress>) -> Self {
        static REQUEST_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        
        Self {
            request_id: REQUEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            addresses,
            max_size: None,
        }
    }
    
    /// Create request for a single chunk
    pub fn single(address: ContentAddress) -> Self {
        Self::new(vec![address])
    }
    
    /// Set maximum response size
    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = Some(max_size);
        self
    }
}

/// Single chunk with its data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    /// Content address (hash) of this chunk
    pub address: ContentAddress,
    
    /// Raw chunk data
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl ChunkData {
    /// Create new chunk data
    pub fn new(address: ContentAddress, data: Vec<u8>) -> Self {
        Self { address, data }
    }
    
    /// Verify the chunk's integrity
    pub fn verify(&self) -> bool {
        let computed = ContentAddress::from_data(&self.data);
        computed == self.address
    }
}

/// Response status for chunk requests
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkResponseStatus {
    /// All requested chunks provided
    Complete,
    
    /// Some chunks provided, some not found
    Partial,
    
    /// No chunks found
    NotFound,
    
    /// Request too large
    TooLarge,
    
    /// Server error
    Error,
}

/// Response containing requested chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResponse {
    /// Request ID this responds to
    pub request_id: u64,
    
    /// Response status
    pub status: ChunkResponseStatus,
    
    /// Retrieved chunks
    pub chunks: Vec<ChunkData>,
    
    /// Addresses that were not found
    pub not_found: Vec<ContentAddress>,
    
    /// Optional error message
    pub error: Option<String>,
}

impl ChunkResponse {
    /// Create a successful response with all chunks
    pub fn complete(request_id: u64, chunks: Vec<ChunkData>) -> Self {
        Self {
            request_id,
            status: ChunkResponseStatus::Complete,
            chunks,
            not_found: Vec::new(),
            error: None,
        }
    }
    
    /// Create a partial response (some chunks found)
    pub fn partial(request_id: u64, chunks: Vec<ChunkData>, not_found: Vec<ContentAddress>) -> Self {
        Self {
            request_id,
            status: ChunkResponseStatus::Partial,
            chunks,
            not_found,
            error: None,
        }
    }
    
    /// Create a not found response
    pub fn not_found(request_id: u64, addresses: Vec<ContentAddress>) -> Self {
        Self {
            request_id,
            status: ChunkResponseStatus::NotFound,
            chunks: Vec::new(),
            not_found: addresses,
            error: None,
        }
    }
    
    /// Create an error response
    pub fn error(request_id: u64, error: String) -> Self {
        Self {
            request_id,
            status: ChunkResponseStatus::Error,
            chunks: Vec::new(),
            not_found: Vec::new(),
            error: Some(error),
        }
    }
    
    /// Total bytes in the response
    pub fn total_bytes(&self) -> usize {
        self.chunks.iter().map(|c| c.data.len()).sum()
    }
}

/// Codec for chunk protocol messages
#[derive(Debug, Clone, Default)]
pub struct ChunkCodec {
    /// Maximum message size (default 16MB)
    pub max_message_size: usize,
}

impl ChunkCodec {
    /// Create a new codec with default settings
    pub fn new() -> Self {
        Self {
            max_message_size: 16 * 1024 * 1024, // 16MB
        }
    }
    
    /// Create codec with custom max size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            max_message_size: max_size,
        }
    }
}

#[async_trait::async_trait]
impl Codec for ChunkCodec {
    type Protocol = StreamProtocol;
    type Request = ChunkRequest;
    type Response = ChunkResponse;
    
    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        if len > self.max_message_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message too large: {} > {}", len, self.max_message_size),
            ));
        }
        
        // Read message body
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        
        // Deserialize
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    
    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        if len > self.max_message_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message too large: {} > {}", len, self.max_message_size),
            ));
        }
        
        // Read message body
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        
        // Deserialize
        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    
    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Serialize
        let data = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        if data.len() > self.max_message_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Request too large",
            ));
        }
        
        // Write length prefix + data
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        
        Ok(())
    }
    
    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Serialize
        let data = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        if data.len() > self.max_message_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Response too large",
            ));
        }
        
        // Write length prefix + data
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        
        Ok(())
    }
}

/// Type alias for chunk protocol behaviour
pub type ChunkProtocol = request_response::Behaviour<ChunkCodec>;

/// Create chunk protocol behaviour
pub fn create_chunk_protocol() -> ChunkProtocol {
    let protocols = vec![(
        StreamProtocol::new(CHUNK_PROTOCOL),
        ProtocolSupport::Full,
    )];
    
    let config = request_response::Config::default()
        .with_request_timeout(std::time::Duration::from_secs(60));
    
    request_response::Behaviour::with_codec(ChunkCodec::new(), protocols, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_request() {
        let addr = ContentAddress::from_data(b"test data");
        let request = ChunkRequest::single(addr.clone());
        
        assert_eq!(request.addresses.len(), 1);
        assert_eq!(request.addresses[0], addr);
    }
    
    #[test]
    fn test_chunk_data_verification() {
        let data = b"hello world".to_vec();
        let address = ContentAddress::from_data(&data);
        let chunk = ChunkData::new(address, data);
        
        assert!(chunk.verify());
    }
    
    #[test]
    fn test_chunk_data_verification_failure() {
        let data = b"hello world".to_vec();
        let address = ContentAddress::from_data(b"different data");
        let chunk = ChunkData::new(address, data);
        
        assert!(!chunk.verify());
    }
    
    #[test]
    fn test_response_types() {
        let chunks = vec![ChunkData::new(
            ContentAddress::from_data(b"test"),
            b"test".to_vec(),
        )];
        
        let complete = ChunkResponse::complete(1, chunks.clone());
        assert_eq!(complete.status, ChunkResponseStatus::Complete);
        
        let not_found = ChunkResponse::not_found(2, vec![ContentAddress::from_data(b"missing")]);
        assert_eq!(not_found.status, ChunkResponseStatus::NotFound);
    }
}
