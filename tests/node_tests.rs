use nebula::{node::Node, config::{NodeState, LogLevel}};

#[test]
fn test_node_creation() {
    let node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        LogLevel::Info,
        false
    ).expect("Failed to create node");
    
    assert_eq!(node.address, "127.0.0.1");
    assert_eq!(node.port, 4001);
    assert!(node.storage_dir.to_string_lossy().contains("node"));
    assert!(!node.daemon_mode);
    assert_eq!(*node.get_status(), NodeState::Stopped);
}

#[test]
fn test_node_state_transitions() {
    let mut node = Node::new(
        "127.0.0.1".to_string(),
        4001,
        LogLevel::Info,
        false
    ).expect("Failed to create node");
    
    // Initial state should be Stopped
    assert_eq!(*node.get_status(), NodeState::Stopped);
    assert!(!node.is_running());
    
    // Start the node
    node.start().expect("Failed to start node");
    assert_eq!(*node.get_status(), NodeState::Running);
    assert!(node.is_running());
    
    // Stop the node
    node.stop().expect("Failed to stop node");
    assert_eq!(*node.get_status(), NodeState::Stopped);
    assert!(!node.is_running());
}
