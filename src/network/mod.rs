pub mod behavior;
pub mod swarm;

#[cfg(test)]
mod tests;

pub use behavior::NebulaNetworkBehavior;
pub use swarm::NetworkManager;

use libp2p::PeerId;

/// Network events that can be emitted by the network layer
#[derive(Debug)]
pub enum NetworkEvent {
    /// A new peer has connected
    PeerConnected(PeerId),
    /// A peer has disconnected
    PeerDisconnected(PeerId),
    /// A ping was received from a peer
    PingReceived(PeerId),
    /// A pong was received from a peer
    PongReceived(PeerId),
}

/// Errors that can occur in the network layer
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Transport error: {0}")]
    Transport(#[from] libp2p::TransportError<std::io::Error>),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Invalid multiaddr: {0}")]
    InvalidAddress(String),
    
    #[error("Peer not found: {0}")]
    PeerNotFound(PeerId),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type NetworkResult<T> = Result<T, NetworkError>;
