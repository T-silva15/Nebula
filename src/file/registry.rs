use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::content::ContentAddress;

pub type FileId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: FileId,
    pub original_name: String,
    pub chunk_addresses: Vec<ContentAddress>,
    pub total_size: u64,
    pub created_at: u64, // Unix timestamp
    pub chunk_count: usize,
}

impl FileMetadata {
    pub fn new(
        original_name: String, 
        chunk_addresses: Vec<ContentAddress>, 
        total_size: u64
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            original_name,
            chunk_count: chunk_addresses.len(),
            chunk_addresses,
            total_size,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    pub fn created_time_string(&self) -> String {
        let created_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.created_at);
        if let Ok(system_time) = created_time.duration_since(std::time::UNIX_EPOCH) {
            format!("{} seconds since epoch", system_time.as_secs())
        } else {
            "Unknown".to_string()
        }
    }
    
    pub fn short_id(&self) -> String {
        format!("{:.8}", self.id.to_string().replace('-', ""))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileRegistryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("File not found: {0}")]
    FileNotFound(FileId),
    
    #[error("Registry file is corrupted")]
    CorruptedRegistry,
}

pub type FileRegistryResult<T> = Result<T, FileRegistryError>;

/// Registry for tracking file-level metadata
#[derive(Debug)]
pub struct FileRegistry {
    registry_path: PathBuf,
    files: HashMap<FileId, FileMetadata>,
}

impl FileRegistry {
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> FileRegistryResult<Self> {
        let registry_path = storage_dir.as_ref().join("file_registry.json");
        
        let files = if registry_path.exists() {
            Self::load_registry(&registry_path)?
        } else {
            HashMap::new()
        };
        
        Ok(Self {
            registry_path,
            files,
        })
    }
    
    /// Register a new file and return its metadata
    pub fn register_file(
        &mut self,
        original_name: String,
        chunk_addresses: Vec<ContentAddress>,
        total_size: u64,
    ) -> FileRegistryResult<FileMetadata> {
        let metadata = FileMetadata::new(original_name, chunk_addresses, total_size);
        self.files.insert(metadata.id, metadata.clone());
        self.save_registry()?;
        Ok(metadata)
    }
    
    /// Get file metadata by ID
    pub fn get_file(&self, file_id: &FileId) -> Option<&FileMetadata> {
        self.files.get(file_id)
    }
    
    /// Find a file by its short ID (first 8 characters of UUID without dashes)
    pub fn get_file_by_short_id(&self, short_id: &str) -> Option<&FileMetadata> {
        self.files.values().find(|metadata| metadata.short_id() == short_id)
    }
    
    /// Remove a file from the registry
    pub fn remove_file(&mut self, file_id: &FileId) -> FileRegistryResult<Option<FileMetadata>> {
        let removed = self.files.remove(file_id);
        if removed.is_some() {
            self.save_registry()?;
        }
        Ok(removed)
    }
    
    /// List all registered files
    pub fn list_files(&self) -> Vec<&FileMetadata> {
        self.files.values().collect()
    }
    
    /// Get files count
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
    
    /// Find files by original name (partial match)
    pub fn find_files_by_name(&self, name_pattern: &str) -> Vec<&FileMetadata> {
        self.files
            .values()
            .filter(|metadata| metadata.original_name.contains(name_pattern))
            .collect()
    }
    
    /// Get total size of all registered files
    pub fn total_size(&self) -> u64 {
        self.files.values().map(|f| f.total_size).sum()
    }
    
    /// Save the registry to disk
    fn save_registry(&self) -> FileRegistryResult<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.registry_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let json = serde_json::to_string_pretty(&self.files)?;
        fs::write(&self.registry_path, json)?;
        Ok(())
    }
    
    /// Load the registry from disk
    fn load_registry<P: AsRef<Path>>(path: P) -> FileRegistryResult<HashMap<FileId, FileMetadata>> {
        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }
        
        let files: HashMap<FileId, FileMetadata> = serde_json::from_str(&content)
            .map_err(|_| FileRegistryError::CorruptedRegistry)?;
        
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_file_registry_creation() {
        let temp_dir = TempDir::new().unwrap();
        let registry = FileRegistry::new(temp_dir.path()).unwrap();
        
        assert_eq!(registry.file_count(), 0);
        assert!(registry.list_files().is_empty());
    }
    
    #[test]
    fn test_register_and_retrieve_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut registry = FileRegistry::new(temp_dir.path()).unwrap();
        
        let addresses = vec![]; // Empty for test
        let metadata = registry.register_file(
            "test.txt".to_string(),
            addresses,
            1024
        ).unwrap();
        
        assert_eq!(registry.file_count(), 1);
        
        let retrieved = registry.get_file(&metadata.id).unwrap();
        assert_eq!(retrieved.original_name, "test.txt");
        assert_eq!(retrieved.total_size, 1024);
    }
    
    #[test]
    fn test_registry_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let file_id;
        
        // Create and register a file
        {
            let mut registry = FileRegistry::new(temp_dir.path()).unwrap();
            let addresses = vec![]; // Empty for test
            let metadata = registry.register_file(
                "persistent.txt".to_string(),
                addresses,
                2048
            ).unwrap();
            file_id = metadata.id;
        }
        
        // Create a new registry instance and verify the file persisted
        {
            let registry = FileRegistry::new(temp_dir.path()).unwrap();
            assert_eq!(registry.file_count(), 1);
            
            let retrieved = registry.get_file(&file_id).unwrap();
            assert_eq!(retrieved.original_name, "persistent.txt");
            assert_eq!(retrieved.total_size, 2048);
        }
    }
}
