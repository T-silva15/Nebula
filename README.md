# 🌌 Nebula - Distributed File System

A peer-to-peer distributed file system built in Rust featuring content-addressable storage, file chunking, deduplication, and distributed hash tables.

![Rust](https://img.shields.io/badge/Rust-1.76+-orange.svg)
![Status](https://img.shields.io/badge/Status-In_Development-yellow.svg)
![License](https://img.shields.io/badge/License-MIT-blue.svg)

## 📋 Overview

Nebula is an ambitious distributed file system project implemented in Rust that demonstrates advanced distributed systems concepts including:

- **Content-Addressable Storage**: Files identified by their content hash rather than location
- **Peer-to-Peer Architecture**: Decentralized node network with no single point of failure
- **File Chunking**: Breaking large files into manageable pieces for efficient distribution
- **Distributed Hash Tables**: Kademlia-based DHT for efficient content routing
- **Data Deduplication**: Automatic detection and elimination of redundant data
- **Content Integrity**: SHA-256 based content addressing for cryptographic verification

## 🚧 Project Status & Roadmap

### ✅ Completed Phases

**Phase 1: Foundation & Architecture** ✅
- [x] Project structure and core architecture
- [x] Local node implementation with persistent storage
- [x] CLI interface and configuration system

**Phase 2: Content-Addressable Storage** ✅  
- [x] SHA-256 based content addressing
- [x] FastCDC variable-sized chunking (4-16KB)
- [x] File registry system with UUID-based file management
- [x] Exceptional deduplication (38-81% storage savings)
- [x] Comprehensive testing framework

### 🚧 Upcoming Phases

**Phase 3: Peer-to-Peer Networking** (Next)
- [ ] libp2p integration for node communication
- [ ] Peer discovery and connection management
- [ ] Basic chunk request/response protocol
- [ ] Secure peer authentication and encryption
- [ ] Multi-node chunk transfer and retrieval

**Phase 4: Distributed Hash Table**
- [ ] Kademlia DHT implementation
- [ ] Distributed content location and routing
- [ ] Node join/leave handling
- [ ] DHT-based peer discovery
- [ ] Network partitioning resilience

**Phase 5: Advanced Distributed Features**
- [ ] Cross-network deduplication optimization
- [ ] Intelligent chunk replication strategies
- [ ] Load balancing and performance optimization
- [ ] Network-wide garbage collection
- [ ] Advanced consistency guarantees

## 🛠️ Technology Stack

**Current Implementation**:
- **FastCDC**: Content-defined chunking with variable-sized boundaries
- **SHA-256**: Cryptographic content addressing and integrity verification
- **Serde**: Serialization for file registry and metadata
- **Clap**: Comprehensive command line interface
- **UUID**: File identification and registry management

**Planned Additions**:
- **libp2p**: Peer-to-peer networking framework
- **Tokio**: Asynchronous runtime for network operations
- **S/Kademlia DHT**: Distributed hash table implementation

## 🚀 Getting Started

### Prerequisites
- Rust 1.76 or higher
- Git

### Installation & Usage

**Option 1: Build and Run Locally**
```bash
# Clone the repository
git clone https://github.com/T-silva15/Nebula.git
cd Nebula

# Build the project
cargo build --release

# Run commands using the built binary
./target/release/nebula start
./target/release/nebula put /path/to/file.txt
```

**Option 2: Install System-wide**
```bash
# Install directly from the repository
cargo install --path .

# Now use 'nebula' command anywhere
nebula start
nebula put /path/to/file.txt
nebula stats
```

**Basic Usage Examples**
```bash
# Start a node (interactive mode)
nebula start

# Start a node in background (daemon mode)
nebula start --daemon

# Store a file and get its ID
nebula put document.pdf

# Retrieve a file by ID (full UUID or 8-char short ID)
nebula get a1b2c3d4 retrieved_document.pdf

# View storage statistics and deduplication ratios
nebula stats

# List all stored files with metadata
nebula list-files --verbose
```

### Deduplication Testing
See [DEDUPLICATION_TESTING.md](./DEDUPLICATION_TESTING.md) for comprehensive deduplication results and testing methodology.

## 🗺️ Feature Roadmap (What's Left)

### Phase 3: Peer-to-Peer Networking
**Goal**: Enable multiple nodes to communicate and share chunks

**Core Components**:
- **libp2p Integration**: Implement robust P2P communication layer
- **Peer Discovery**: Bootstrap nodes and peer finding mechanisms  
- **Connection Management**: Handle peer connections, reconnections, and timeouts
- **Chunk Transfer Protocol**: Request/response system for chunk sharing
- **Security Layer**: Peer authentication and encrypted communication
- **Network CLI**: Commands for peer management and network status

**Deliverables**:
- Multi-node chunk sharing functionality
- Distributed file reconstruction across peers
- Network health monitoring and diagnostics
- Cross-node deduplication validation

### Phase 4: Distributed Hash Table
**Goal**: Implement efficient content location and routing

**Core Components**:
- **Kademlia DHT**: Industry-standard DHT for content routing
- **Distributed Indexing**: Chunk location tracking across the network
- **Routing Optimization**: Efficient peer selection and content discovery
- **Network Topology**: Dynamic network structure management
- **Fault Tolerance**: Handle node failures and network partitions

**Deliverables**:
- O(log N) content lookup complexity
- Automatic load balancing across nodes
- Resilient network topology management
- Performance benchmarking and optimization

### Phase 5: Advanced Distributed Features (Q4 2025)
**Goal**: Production-ready distributed file system

**Core Components**:
- **Proper File Browsing**: Simple and clean file browser 
- **Replication Strategies**: Configurable redundancy and availability
- **Cross-Network Deduplication**: Global chunk sharing optimization
- **Consistency Models**: Strong vs eventual consistency options
- **Garbage Collection**: Distributed cleanup of unused chunks
- **Performance Optimization**: Caching, prefetching, and bandwidth management
- **Monitoring & Observability**: Comprehensive system metrics and alerting

**Deliverables**:
- Production-grade reliability and performance
- Configurable replication and consistency guarantees
- Advanced monitoring and operational tooling
- Comprehensive documentation and deployment guides

## 📚 Architecture

### Current Architecture (Phase 2)
- **Node Management**: Local node lifecycle and configuration
- **Content Addressing**: SHA-256 based content identification
- **Storage Layer**: Persistent chunk storage with deduplication
- **Chunking Engine**: FastCDC variable-sized content-defined chunking
- **File Registry**: UUID-based file metadata and reconstruction
- **CLI Interface**: Comprehensive command-line operations

### Planned Architecture (Phase 3+)
- **Network Layer**: libp2p-based peer communication and discovery
- **DHT Layer**: Kademlia distributed hash table for content routing
- **Replication Engine**: Configurable redundancy and fault tolerance
- **Consistency Manager**: Distributed state synchronization

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.
