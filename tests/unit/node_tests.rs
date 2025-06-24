use nebula::{node::Node, config::NodeState};

#[test]
fn test_node_creation() {
    let node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        "/tmp/test".to_string(),
        "info".to_string(),
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
        "info".to_string(),
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
