use serde::{Deserialize, Serialize};
use crate::character::CharacterConfig;

/// Snapshot of a remote player's visible state.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetPlayer {
    pub id:        String,
    pub position:  [f32; 3],
    pub yaw:       f32,
    pub sanity:    f32,
    pub character: CharacterConfig,
}

/// Client → Server messages.
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMsg {
    Join     { id: String, character: CharacterConfig },
    Move     { position: [f32; 3], yaw: f32, pitch: f32 },
    Interact { kind: InteractKind },
    Chat     { text: String },
    Leave,
}

/// Server → Client messages.
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMsg {
    Welcome  { your_id: String, players: Vec<NetPlayer>, rooms_unlocked: Vec<usize> },
    Joined   (NetPlayer),
    Left     { id: String },
    Moved    { id: String, position: [f32; 3], yaw: f32, pitch: f32 },
    Unlocked { room_id: usize },
    ItemTaken{ item_idx: usize, player_id: String },
    Chat     { player_name: String, text: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InteractKind {
    PickupItem { index: usize },
    UseDoor    { index: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Offline,
    Connecting,
    Connected,
    Hosting,
    Failed(String),
}

pub struct NetworkManager {
    pub state:        ConnectionState,
    pub local_id:     String,
    pub players:      Vec<NetPlayer>,
    pub host_address: String,
    pub join_address: String,
    /// Incoming messages from the network layer (filled by tick()).
    pub inbox:  Vec<ServerMsg>,
    /// Outgoing messages queued for sending (drained by tick()).
    pub outbox: Vec<ClientMsg>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            state:        ConnectionState::Offline,
            local_id:     simple_uuid(),
            players:      Vec::new(),
            host_address: String::from("0.0.0.0:7777"),
            join_address: String::from("127.0.0.1:7777"),
            inbox:        Vec::new(),
            outbox:       Vec::new(),
        }
    }

    pub fn host(&mut self) {
        // TODO: bind TCP/UDP listener on self.host_address
        self.state = ConnectionState::Hosting;
    }

    pub fn connect(&mut self) {
        // TODO: TCP/UDP connect to self.join_address
        self.state = ConnectionState::Connecting;
    }

    pub fn disconnect(&mut self) {
        self.outbox.push(ClientMsg::Leave);
        self.state = ConnectionState::Offline;
        self.players.clear();
    }

    /// Poll network, flush outbox, fill inbox. No-op until networking is wired up.
    pub fn tick(&mut self) {
        // TODO: actual async network poll via tokio
    }

    pub fn send(&mut self, msg: ClientMsg) {
        self.outbox.push(msg);
    }

    pub fn is_online(&self) -> bool {
        matches!(self.state, ConnectionState::Connected | ConnectionState::Hosting)
    }
}

fn simple_uuid() -> String {
    use rand::Rng;
    let mut r = rand::thread_rng();
    let b: [u8; 16] = r.gen();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:01x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],b[1],b[2],b[3], b[4],b[5], b[6]&0xf,b[7],
        (b[8]&0x3f)|0x80,b[9], b[10],b[11],b[12],b[13],b[14],b[15]
    )
}
