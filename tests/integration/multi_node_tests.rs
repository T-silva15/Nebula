use nebula::{config::Config, node::Node};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Multi-node test environment for distributed testing
pub struct MultiNodeTest {
    pub nodes: Vec<Arc<Mutex<Node>>>,
    pub configs: Vec<Config>,
}

impl MultiNodeTest {
    /// Create a test environment with multiple nodes
    pub async fn new(node_count: usize) -> Self {
        let mut nodes = Vec::new();
        let mut configs = Vec::new();
        
        for i in 0..node_count {
            let config = Config {
                listen_port: 4000 + i as u16, // Different ports for each node
                listen_address: "127.0.0.1".to_string(),
                storage_dir: tempfile::tempdir().unwrap().into_path(),
                log_level: nebula::config::LogLevel::Error, // Quiet during tests
                daemon_mode: false,
                verbose: false,
            };
            
            let node = Node::new(
                config.listen_address.clone(),
                config.listen_port,
                config.storage_dir.to_string_lossy().to_string(),
                config.log_level.to_string(),
                config.daemon_mode,
            );
            
            nodes.push(Arc::new(Mutex::new(node)));
            configs.push(config);
        }
        
        Self { nodes, configs }
    }
    
    /// Start all nodes
    pub async fn start_all(&self) {
        for node in &self.nodes {
            let mut node = node.lock().await;
            node.start();
        }
    }
    
    /// Stop all nodes
    pub async fn stop_all(&self) {
        for node in &self.nodes {
            let mut node = node.lock().await;
            node.stop();
        }
    }
    
    /// Get node by index
    pub fn get_node(&self, index: usize) -> Option<&Arc<Mutex<Node>>> {
        self.nodes.get(index)
    }
}

#[tokio::test]
async fn test_multiple_nodes_startup() {
    let test_env = MultiNodeTest::new(3).await;
    
    // Start all nodes
    test_env.start_all().await;
    
    // Verify all nodes are running
    for node in &test_env.nodes {
        let node = node.lock().await;
        assert!(node.is_running());
    }
    
    // Stop all nodes
    test_env.stop_all().await;
    
    // Verify all nodes are stopped
    for node in &test_env.nodes {
        let node = node.lock().await;
        assert!(!node.is_running());
    }
}
