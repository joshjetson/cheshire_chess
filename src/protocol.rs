use serde::{Deserialize, Serialize};

// ── Client → Server ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    SetName { name: String },
    ListRooms,
    CreateRoom { name: String },
    JoinRoom { room_id: u32 },
    LeaveRoom,

    // Game tables within a room
    CreateTable,                         // create a table, wait for opponent
    JoinTable { table_id: u32 },         // sit down to play or spectate
    LeaveTable,                          // stand up from table

    // Moves (applies to whichever table you're at)
    MakeMove { uci: String },
    Resign,

    // Main board (moderator only)
    SetMainBoardMode { mode: BoardMode },
    MainBoardMove { uci: String },

    // Chat (goes to the whole room)
    SendChat { body: String },
}

// ── Server → Client ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    Welcome { your_id: u32 },
    Error { msg: String },

    // Room browsing
    RoomList { rooms: Vec<RoomInfo> },

    // Room state
    RoomJoined { room: RoomInfo, players: Vec<PlayerInfo>, tables: Vec<TableInfo> },
    PlayerJoined { player: PlayerInfo },
    PlayerLeft { player_id: u32 },

    // Game tables
    TableCreated { table: TableInfo },
    TableUpdated { table: TableInfo },
    TableRemoved { table_id: u32 },
    TableJoined { table: TableInfo, fen: String },

    // Game events (within a table)
    GameStarted { table_id: u32, white: u32, black: u32, fen: String },
    MoveMade { table_id: u32, uci: String, fen: String },
    GameOver { table_id: u32, reason: String, winner: Option<u32> },

    // Main board
    MainBoardUpdate { mode: BoardMode, fen: String },

    // Chat
    ChatMessage { sender: String, body: String, kind: ChatKind },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: u32,
    pub name: String,
    pub player_count: u32,
    pub table_count: u32,
    pub active_games: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: u32,
    pub name: String,
    pub status: PlayerStatus,
    pub table_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlayerStatus {
    Idle,
    Playing,
    Spectating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub id: u32,
    pub white: Option<PlayerRef>,
    pub black: Option<PlayerRef>,
    pub spectator_count: u32,
    pub has_game: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRef {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BoardMode {
    Tutorial,
    Game,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatKind {
    Player,
    System,
    Spectator,
}

pub const DEFAULT_PORT: u16 = 7878;
