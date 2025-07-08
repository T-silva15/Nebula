use libp2p::{
    noise, tcp, yamux,
    swarm::Swarm, SwarmBuilder,
    PeerId, Multiaddr,
    identity::Keypair,
};
use tokio::sync::mpsc;
use futures::StreamExt;

use super::{NebulaNetworkBehavior, NetworkEvent, NetworkError, NetworkResult};
use super::behavior::NebulaNetworkEvent;

/// Manages the libp2p network swarm and connections
pub struct NetworkManager {
    /// The libp2p swarm
    swarm: Swarm<NebulaNetworkBehavior>,
    /// Event sender for communicating with the application
    event_sender: mpsc::UnboundedSender<NetworkEvent>,
    /// Our local peer ID
    local_peer_id: PeerId,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new() -> NetworkResult<(Self, mpsc::UnboundedReceiver<NetworkEvent>)> {
        // Generate a keypair for this node
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());
        
        println!("Local peer ID: {}", local_peer_id);

        // Create network behavior
        let behavior = NebulaNetworkBehavior::new(local_peer_id, keypair.public());

        // Create the swarm with the new API
        let swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| NetworkError::Connection(e.to_string()))?
            .with_behaviour(|_| behavior)
            .map_err(|e| NetworkError::Connection(e.to_string()))?
            .build();

        // Create event channel
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let manager = NetworkManager {
            swarm,
            event_sender,
            local_peer_id,
        };

        Ok((manager, event_receiver))
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Start listening on the given address
    pub fn listen_on(&mut self, addr: Multiaddr) -> NetworkResult<()> {
        self.swarm.listen_on(addr)
            .map_err(|e| NetworkError::Transport(e))?;
        
        println!("Listening on address");
        Ok(())
    }

    /// Connect to a remote peer
    pub fn dial(&mut self, addr: Multiaddr) -> NetworkResult<()> {
        self.swarm.dial(addr)
            .map_err(|e| NetworkError::Connection(e.to_string()))?;
        
        println!("Dialing peer");
        Ok(())
    }

    /// Get the list of connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.swarm.connected_peers().cloned().collect()
    }

    /// Main event loop for the network manager
    pub async fn run(&mut self) {
        loop {
            match self.swarm.next().await {
                Some(libp2p::swarm::SwarmEvent::NewListenAddr { address, .. }) => {
                    println!("Listening on {}", address);
                }
                Some(libp2p::swarm::SwarmEvent::Behaviour(event)) => {
                    self.handle_behavior_event(event);
                }
                Some(libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. }) => {
                    println!("Connected to peer: {}", peer_id);
                    let _ = self.event_sender.send(NetworkEvent::PeerConnected(peer_id));
                }
                Some(libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. }) => {
                    println!("Disconnected from peer: {}", peer_id);
                    let _ = self.event_sender.send(NetworkEvent::PeerDisconnected(peer_id));
                }
                Some(_) => {}
                None => break,
            }
        }
    }

    /// Handle behavior-specific events
    fn handle_behavior_event(&self, event: NebulaNetworkEvent) {
        match event {
            NebulaNetworkEvent::Ping(ping_event) => {
                match ping_event {
                    libp2p::ping::Event {
                        peer,
                        result: Ok(rtt),
                        ..
                    } => {
                        println!("Ping to {} succeeded with RTT: {:?}", peer, rtt);
                        let _ = self.event_sender.send(NetworkEvent::PongReceived(peer));
                    }
                    libp2p::ping::Event {
                        peer,
                        result: Err(err),
                        ..
                    } => {
                        println!("Ping to {} failed: {:?}", peer, err);
                    }
                }
            }
            NebulaNetworkEvent::Identify(identify_event) => {
                match identify_event {
                    libp2p::identify::Event::Received { peer_id, info, .. } => {
                        println!("Received identify info from {}: {:?}", peer_id, info);
                    }
                    libp2p::identify::Event::Sent { peer_id, .. } => {
                        println!("Sent identify info to {}", peer_id);
                    }
                    _ => {}
                }
            }
        }
    }
}
