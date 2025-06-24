
pub struct Node {
    pub id: String,             // Generated or loaded from storage
    pub state: String,          // Node state
    pub address: String,        // Node's network address
    pub port: u16,              // Listening port
    pub storage_dir: String,    // Directory for storing data
    pub log_level: String,      // Logging level (error, warn, info, debug, trace)
    pub daemon_mode: bool,      // Whether to run as a daemon
    

    }