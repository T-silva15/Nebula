//! Combined network behaviour
//!
//! Combines all libp2p protocols into a single NetworkBehaviour.

use libp2p::{
    identify,
    kad::{self, store::MemoryStore},
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
    
    /// Kademlia DHT for distributed content discovery
    pub kademlia: kad::Behaviour<MemoryStore>,
}

/// Events emitted by the combined behaviour
#[derive(Debug)]
pub enum NebulaBehaviourEvent {
    Ping(ping::Event),
    Identify(identify::Event),
    Mdns(mdns::Event),
    ChunkProtocol(request_response::Event<ChunkRequest, ChunkResponse>),
    Kademlia(kad::Event),
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

impl From<kad::Event> for NebulaBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        NebulaBehaviourEvent::Kademlia(event)
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
        
        // Kademlia DHT for distributed content discovery
        // Using S/Kademlia parameters: k=20 replication factor
        let store = MemoryStore::new(local_peer_id);
        let mut kad_config = kad::Config::new(libp2p::StreamProtocol::new("/nebula/kad/1.0.0"));
        kad_config.set_replication_factor(std::num::NonZeroUsize::new(20).unwrap());
        let kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);
        
        Ok(Self {
            ping,
            identify,
            mdns,
            chunk_protocol,
            kademlia,
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
    
    /// Add a peer address to the Kademlia routing table
    pub fn add_address(&mut self, peer: &PeerId, addr: libp2p::Multiaddr) {
        self.kademlia.add_address(peer, addr);
    }
    
    /// Bootstrap the Kademlia DHT
    pub fn bootstrap(&mut self) -> Result<kad::QueryId, kad::NoKnownPeers> {
        self.kademlia.bootstrap()
    }
    
    /// Start providing a content key
    pub fn start_providing(&mut self, key: kad::RecordKey) -> Result<kad::QueryId, kad::store::Error> {
        self.kademlia.start_providing(key)
    }
    
    /// Get providers for a content key
    pub fn get_providers(&mut self, key: kad::RecordKey) -> kad::QueryId {
        self.kademlia.get_providers(key)
    }
    
    /// Store a record in the DHT
    pub fn put_record(&mut self, record: kad::Record, quorum: kad::Quorum) -> Result<kad::QueryId, kad::store::Error> {
        self.kademlia.put_record(record, quorum)
    }
    
    /// Get a record from the DHT
    pub fn get_record(&mut self, key: kad::RecordKey) -> kad::QueryId {
        self.kademlia.get_record(key)
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
