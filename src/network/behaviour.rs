//! Combined network behaviour
//!
//! Combines all libp2p protocols into a single NetworkBehaviour.

use libp2p::{
    identify,
    mdns,
    ping,
    request_response,
    swarm::NetworkBehaviour,
    PeerId,
};

use super::protocol::{ChunkProtocol, ChunkRequest, ChunkResponse};

/// Combined network behaviour for Nebula
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NebulaBehaviourEvent")]
pub struct NebulaBehaviour {
    /// Ping protocol for connection health
    pub ping: ping::Behaviour,
    
    /// Identify protocol for peer information exchange
    pub identify: identify::Behaviour,
    
    /// mDNS for local peer discovery
    pub mdns: mdns::tokio::Behaviour,
    
    /// Chunk transfer protocol
    pub chunk_protocol: ChunkProtocol,
}

/// Events emitted by the combined behaviour
#[derive(Debug)]
pub enum NebulaBehaviourEvent {
    Ping(ping::Event),
    Identify(identify::Event),
    Mdns(mdns::Event),
    ChunkProtocol(request_response::Event<ChunkRequest, ChunkResponse>),
}

impl From<ping::Event> for NebulaBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        NebulaBehaviourEvent::Ping(event)
    }
}

impl From<identify::Event> for NebulaBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        NebulaBehaviourEvent::Identify(event)
    }
}

impl From<mdns::Event> for NebulaBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        NebulaBehaviourEvent::Mdns(event)
    }
}

impl From<request_response::Event<ChunkRequest, ChunkResponse>> for NebulaBehaviourEvent {
    fn from(event: request_response::Event<ChunkRequest, ChunkResponse>) -> Self {
        NebulaBehaviourEvent::ChunkProtocol(event)
    }
}

impl NebulaBehaviour {
    /// Create a new NebulaBehaviour
    pub fn new(local_peer_id: PeerId, local_public_key: libp2p::identity::PublicKey) -> Result<Self, Box<dyn std::error::Error>> {
        // Ping behaviour for connection health
        let ping = ping::Behaviour::new(ping::Config::new());
        
        // Identify behaviour for peer info exchange
        let identify = identify::Behaviour::new(
            identify::Config::new(
                format!("/nebula/{}", env!("CARGO_PKG_VERSION")),
                local_public_key,
            )
            .with_agent_version(format!("nebula/{}", env!("CARGO_PKG_VERSION"))),
        );
        
        // mDNS for local discovery
        let mdns_config = mdns::Config {
            ttl: std::time::Duration::from_secs(300),
            query_interval: std::time::Duration::from_secs(30),
            enable_ipv6: false,
        };
        let mdns = mdns::tokio::Behaviour::new(mdns_config, local_peer_id)?;
        
        // Chunk transfer protocol
        let chunk_protocol = super::protocol::create_chunk_protocol();
        
        Ok(Self {
            ping,
            identify,
            mdns,
            chunk_protocol,
        })
    }
    
    /// Send a chunk request to a peer
    pub fn request_chunks(&mut self, peer: &PeerId, request: ChunkRequest) -> request_response::OutboundRequestId {
        self.chunk_protocol.send_request(peer, request)
    }
    
    /// Send a chunk response
    pub fn send_response(
        &mut self,
        channel: request_response::ResponseChannel<ChunkResponse>,
        response: ChunkResponse,
    ) -> Result<(), ChunkResponse> {
        self.chunk_protocol.send_response(channel, response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    
    #[tokio::test]
    async fn test_behaviour_creation() {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        
        let behaviour = NebulaBehaviour::new(peer_id, keypair.public());
        assert!(behaviour.is_ok());
    }
}
