//! Network transport layer using libp2p
//!
//! Configures TCP transport with Noise encryption and Yamux multiplexing.

use libp2p::{
    core::upgrade,
    identity::Keypair,
    noise,
    tcp,
    yamux,
    Transport,
    PeerId,
};
use std::time::Duration;

use super::NetworkError;

/// Configuration for the network transport
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable TCP nodelay (disable Nagle's algorithm)
    pub tcp_nodelay: bool,
    /// Maximum number of connections per peer
    pub max_connections_per_peer: u32,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(30),
            tcp_nodelay: true,
            max_connections_per_peer: 2,
        }
    }
}

/// Wrapper around libp2p transport configuration
pub struct NetworkTransport {
    config: TransportConfig,
}

impl NetworkTransport {
    /// Create a new transport with default configuration
    pub fn new() -> Self {
        Self {
            config: TransportConfig::default(),
        }
    }
    
    /// Create a new transport with custom configuration
    pub fn with_config(config: TransportConfig) -> Self {
        Self { config }
    }
    
    /// Build the libp2p transport stack
    /// 
    /// Creates a transport with:
    /// - TCP as the underlying transport
    /// - Noise protocol for encryption
    /// - Yamux for stream multiplexing
    pub fn build(
        &self,
        keypair: &Keypair,
    ) -> Result<libp2p::core::transport::Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>, NetworkError> {
        // Create TCP transport
        let tcp_config = tcp::Config::default()
            .nodelay(self.config.tcp_nodelay);
        
        let tcp_transport = tcp::tokio::Transport::new(tcp_config);
        
        // Create Noise configuration for encryption
        let noise_config = noise::Config::new(keypair)
            .map_err(|e| NetworkError::Transport(format!("Noise config error: {}", e)))?;
        
        // Create Yamux configuration for multiplexing
        let yamux_config = yamux::Config::default();
        
        // Build the transport stack
        let transport = tcp_transport
            .upgrade(upgrade::Version::V1)
            .authenticate(noise_config)
            .multiplex(yamux_config)
            .timeout(self.config.connection_timeout)
            .boxed();
        
        Ok(transport)
    }
}

impl Default for NetworkTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    
    #[test]
    fn test_transport_creation() {
        let transport = NetworkTransport::new();
        let keypair = Keypair::generate_ed25519();
        
        // Should successfully create transport
        let result = transport.build(&keypair);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_custom_config() {
        let config = TransportConfig {
            connection_timeout: Duration::from_secs(60),
            tcp_nodelay: false,
            max_connections_per_peer: 4,
        };
        
        let transport = NetworkTransport::with_config(config.clone());
        assert_eq!(transport.config.connection_timeout, Duration::from_secs(60));
        assert_eq!(transport.config.tcp_nodelay, false);
    }
}
