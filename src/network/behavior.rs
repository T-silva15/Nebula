use libp2p::{
    identify, ping,
    swarm::NetworkBehaviour,
    PeerId,
};

/// Network behavior for Nebula nodes
/// Combines multiple protocols: identify (peer info) and ping (connectivity)
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NebulaNetworkEvent")]
pub struct NebulaNetworkBehavior {
    /// Ping protocol for testing connectivity
    pub ping: ping::Behaviour,
    /// Identify protocol for peer information exchange
    pub identify: identify::Behaviour,
}

/// Events that our network behavior can emit
#[derive(Debug)]
pub enum NebulaNetworkEvent {
    Ping(ping::Event),
    Identify(identify::Event),
}

impl From<ping::Event> for NebulaNetworkEvent {
    fn from(event: ping::Event) -> Self {
        NebulaNetworkEvent::Ping(event)
    }
}

impl From<identify::Event> for NebulaNetworkEvent {
    fn from(event: identify::Event) -> Self {
        NebulaNetworkEvent::Identify(event)
    }
}

impl NebulaNetworkBehavior {
    /// Create a new network behavior
    pub fn new(_local_peer_id: PeerId, local_public_key: libp2p::identity::PublicKey) -> Self {
        // Create identify protocol
        let identify = identify::Behaviour::new(
            identify::Config::new(
                "/nebula/1.0.0".to_string(), // Protocol version
                local_public_key,
            )
        );

        // Create ping protocol with default config
        let ping = ping::Behaviour::new(ping::Config::new());

        Self { ping, identify }
    }
}
