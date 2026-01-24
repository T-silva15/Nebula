# 🌌 Nebula

A peer-to-peer distributed file system built in Rust.

![Rust](https://img.shields.io/badge/Rust-1.76+-orange.svg)
![Status](https://img.shields.io/badge/Status-Complete-green.svg)
![Tests](https://img.shields.io/badge/Tests-72_passing-brightgreen.svg)
![License](https://img.shields.io/badge/License-MIT-blue.svg)

## Features

- **Content-Addressable Storage** - Files identified by SHA-256 hash
- **P2P Networking** - libp2p with mDNS discovery and TCP transport
- **Distributed Hash Table** - S/Kademlia (k=20, α=3) for content routing
- **Content-Defined Chunking** - FastCDC for efficient deduplication
- **Automatic Deduplication** - 38-81% storage savings

## Quick Start

```bash
# Build
cargo build --release

# Start a node
./target/release/nebula start --port 4001

# Store a file
./target/release/nebula put /path/to/file.txt

# Get file info
./target/release/nebula list-files

# View storage stats
./target/release/nebula stats
```

## Multi-Node Setup

Start multiple nodes on different ports - they'll discover each other automatically via mDNS:

```bash
# Terminal 1
nebula start --port 4001 --storage /tmp/node1

# Terminal 2
nebula start --port 4002 --storage /tmp/node2
```

Nodes automatically:
- Discover peers via mDNS
- Exchange peer info via Identify protocol
- Update Kademlia DHT routing tables
- Ping to verify connectivity

## CLI Commands

### Node Management
```bash
nebula start [--port PORT] [--storage PATH]  # Start node
nebula status                                 # Show node status
nebula stop                                   # Stop running node
```

### File Operations
```bash
nebula put <file>                  # Store a file
nebula get <file_id> <output>      # Retrieve a file
nebula list                        # List stored chunks
nebula list-files                  # List registered files
nebula stats                       # Storage statistics
```

### Network Commands
```bash
nebula network status              # Network status
nebula network id                  # Show local peer ID
nebula peer list                   # List connected peers
nebula peer connect <multiaddr>    # Connect to a peer
```

### DHT Commands
```bash
nebula dht stats                   # DHT statistics
nebula dht publish <address>       # Announce content
nebula dht find <address>          # Find content providers
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Node                                │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ File Registry│  │Content Store │  │   Network    │       │
│  │  (metadata)  │  │  (chunks)    │  │   (libp2p)   │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │ 
│                                              │              |
│                                    ┌─────────┴─────────┐    │
│                                    │                   │    │
│                              ┌─────┴─────┐     ┌───────┴──┐ │
│                              │ Kademlia  │     │  mDNS    │ │
│                              │   DHT     │     │Discovery │ │
│                              └───────────┘     └──────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Tech Stack

- **libp2p** - P2P networking (TCP, Noise, Yamux, Kademlia, mDNS)
- **tokio** - Async runtime
- **FastCDC** - Content-defined chunking
- **SHA-256** - Content addressing
- **serde** - Serialization

## Development

```bash
# Run tests
cargo test

# Run with verbose output
cargo run -- start --verbose

# Build release
cargo build --release
```

## License

MIT License - see [LICENSE](LICENSE) for details.
