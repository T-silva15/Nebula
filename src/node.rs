use uuid::Uuid;
use crate::config::NodeState;

pub struct Node {
    pub id: Uuid,               // Generated or loaded from storage
    pub state: NodeState,       // Node state (using enum)
    pub address: String,        // Node's network address
    pub port: u16,              // Listening port
    pub storage_dir: String,    // Directory for storing data
    pub log_level: String,      // Logging level (error, warn, info, debug, trace)
    pub daemon_mode: bool,      // Whether to run as a daemon
}

impl Node {
    pub fn new(address: String, port: u16, storage_dir: String, log_level: String, daemon_mode: bool) -> Node {
        Node {
            id: Uuid::new_v4(),
            state: NodeState::Stopped,  // Start as stopped, will change to Starting/Running
            address,
            port,
            storage_dir,
            log_level,
            daemon_mode,
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