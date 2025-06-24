use uuid::Uuid;
use crate::config::{NodeState, LogLevel};

pub struct Node {
    pub id: Uuid,               // Generated or loaded from storage
    pub state: NodeState,       // Node state (enum)
    pub address: String,        // Node's network address
    pub port: u16,              // Listening port
    pub storage_dir: String,    // Directory for storing data
    pub log_level: LogLevel,    // Logging level (using enum)
    pub daemon_mode: bool,      // Whether to run as a daemon
}

impl Node {
    pub fn new(address: String, port: u16, storage_dir: String, log_level: LogLevel, daemon_mode: bool) -> Node {
        Node {
            id: Uuid::new_v4(),
            state: NodeState::Stopped,
            address: address,
            port: port,
            storage_dir: storage_dir,
            log_level: log_level,
            daemon_mode: daemon_mode,
        }
    }

    pub fn start(&mut self) {
        self.state = NodeState::Starting;
        // TODO: Actual startup logic here
        self.state = NodeState::Running;
    }

    pub fn stop(&mut self) {
        self.state = NodeState::Stopping;
        // TODO: Actual shutdown logic here  
        self.state = NodeState::Stopped;
    }
    
    pub fn is_running(&self) -> bool {
        matches!(self.state, NodeState::Running)
    }
    
    pub fn get_status(&self) -> &NodeState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(
            "127.0.0.1".to_string(),
            4001,
            "/tmp/test".to_string(),
            LogLevel::Info,
            false
        );
        
        assert_eq!(node.address, "127.0.0.1");
        assert_eq!(node.port, 4001);
        assert_eq!(node.storage_dir, "/tmp/test");
        assert!(!node.daemon_mode);
        assert_eq!(*node.get_status(), NodeState::Stopped);
    }

    #[test]
    fn test_node_state_transitions() {
        let mut node = Node::new(
            "127.0.0.1".to_string(),
            4001,
            "/tmp/test".to_string(),
            LogLevel::Info,
            false
        );
        
        // Initial state should be Stopped
        assert_eq!(*node.get_status(), NodeState::Stopped);
        assert!(!node.is_running());
        
        // Start the node
        node.start();
        assert_eq!(*node.get_status(), NodeState::Running);
        assert!(node.is_running());
        
        // Stop the node
        node.stop();
        assert_eq!(*node.get_status(), NodeState::Stopped);
        assert!(!node.is_running());
    }
}