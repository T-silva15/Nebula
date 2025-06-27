use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fmt;
use std::str::FromStr;

/// Hash algorithm used for content addressing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HashAlgorithm {
    Sha256,
    Blake3,
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        HashAlgorithm::Sha256
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashAlgorithm::Sha256 => write!(f, "sha256"),
            HashAlgorithm::Blake3 => write!(f, "blake3"),
        }
    }
}

/// Content address based on cryptographic hash
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentAddress {
    hash: [u8; 32],     // 256-bit hash
    algorithm: HashAlgorithm,
}

impl ContentAddress {
    /// Create content address from raw data
    pub fn from_data(data: &[u8]) -> Self {
        Self::from_data_with_algorithm(data, HashAlgorithm::default())
    }
    
    /// Create content address with specific algorithm
    pub fn from_data_with_algorithm(data: &[u8], algorithm: HashAlgorithm) -> Self {
        let hash = match algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                let result = hasher.finalize();
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&result);
                hash_bytes
            },
            HashAlgorithm::Blake3 => {
                let hash = blake3::hash(data);
                *hash.as_bytes()
            },
        };
        
        Self { hash, algorithm }
    }
    
    /// Convert to hexadecimal string representation
    pub fn to_hex(&self) -> String {
        format!("{}:{}", self.algorithm, hex::encode(self.hash))
    }
    
    /// Parse from hexadecimal string representation
    pub fn from_hex(hex_str: &str) -> Result<Self, ContentAddressError> {
        let parts: Vec<&str> = hex_str.split(':').collect();
        if parts.len() != 2 {
            return Err(ContentAddressError::InvalidFormat);
        }
        
        let algorithm = match parts[0] {
            "sha256" => HashAlgorithm::Sha256,
            "blake3" => HashAlgorithm::Blake3,
            _ => return Err(ContentAddressError::UnsupportedAlgorithm),
        };
        
        let hash_bytes = hex::decode(parts[1])
            .map_err(|_| ContentAddressError::InvalidHex)?;
            
        if hash_bytes.len() != 32 {
            return Err(ContentAddressError::InvalidHashLength);
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);
        
        Ok(Self { hash, algorithm })
    }
    
    /// Get the raw hash bytes
    pub fn hash_bytes(&self) -> &[u8; 32] {
        &self.hash
    }
    
    /// Get the hash algorithm
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }
}

impl fmt::Display for ContentAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for ContentAddress {
    type Err = ContentAddressError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

/// Errors that can occur when working with content addresses
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentAddressError {
    InvalidFormat,
    UnsupportedAlgorithm,
    InvalidHex,
    InvalidHashLength,
}

impl fmt::Display for ContentAddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentAddressError::InvalidFormat => write!(f, "Invalid content address format"),
            ContentAddressError::UnsupportedAlgorithm => write!(f, "Unsupported hash algorithm"),
            ContentAddressError::InvalidHex => write!(f, "Invalid hexadecimal encoding"),
            ContentAddressError::InvalidHashLength => write!(f, "Invalid hash length"),
        }
    }
}

impl std::error::Error for ContentAddressError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_address_creation() {
        let data = b"hello world";
        let addr = ContentAddress::from_data(data);
        
        // Should be deterministic
        let addr2 = ContentAddress::from_data(data);
        assert_eq!(addr, addr2);
        
        // Different data should have different addresses
        let addr3 = ContentAddress::from_data(b"different data");
        assert_ne!(addr, addr3);
    }
    
    #[test]
    fn test_hex_roundtrip() {
        let data = b"test data for roundtrip";
        let addr = ContentAddress::from_data(data);
        
        let hex = addr.to_hex();
        let parsed = ContentAddress::from_hex(&hex).unwrap();
        
        assert_eq!(addr, parsed);
    }
    
    #[test]
    fn test_different_algorithms() {
        let data = b"algorithm test";
        
        let sha256_addr = ContentAddress::from_data_with_algorithm(data, HashAlgorithm::Sha256);
        let blake3_addr = ContentAddress::from_data_with_algorithm(data, HashAlgorithm::Blake3);
        
        // Same data, different algorithms should produce different addresses
        assert_ne!(sha256_addr, blake3_addr);
        assert_eq!(sha256_addr.algorithm(), HashAlgorithm::Sha256);
        assert_eq!(blake3_addr.algorithm(), HashAlgorithm::Blake3);
    }
    
    #[test]
    fn test_invalid_hex_parsing() {
        assert!(ContentAddress::from_hex("invalid").is_err());
        assert!(ContentAddress::from_hex("sha256:invalid_hex").is_err());
        assert!(ContentAddress::from_hex("unknown:deadbeef").is_err());
    }
}
