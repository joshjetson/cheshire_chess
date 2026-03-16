#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;

use crate::board::{self, Move, Position};
use crate::protocol::*;

// ── Server State ───────────────────────────────────────────────────

struct ServerState {
    next_id: u32,
    players: HashMap<u32, ConnectedPlayer>,
    rooms: HashMap<u32, Room>,
}

struct ConnectedPlayer {
    id: u32,
    name: String,
    room_id: Option<u32>,
    table_id: Option<u32>,
    tx: mpsc::UnboundedSender<ServerMsg>,
}

struct Room {
    id: u32,
    name: String,
    moderator: u32,
    player_ids: Vec<u32>,
    tables: HashMap<u32, GameTable>,
    next_table_id: u32,
    main_board_mode: BoardMode,
    main_board_pos: Position,
}

struct GameTable {
    id: u32,
    white: Option<u32>,
    black: Option<u32>,
    spectators: Vec<u32>,
    position: Position,
    game_active: bool,
}

impl ServerState {
    fn new() -> Self {
        Self {
            next_id: 1,
            players: HashMap::new(),
            rooms: HashMap::new(),
        }
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn send_to(&self, pid: u32, msg: ServerMsg) {
        if let Some(p) = self.players.get(&pid) {
            let _ = p.tx.send(msg);
        }
    }

    fn broadcast_room(&self, room_id: u32, msg: ServerMsg, exclude: Option<u32>) {
        if let Some(room) = self.rooms.get(&room_id) {
            for &pid in &room.player_ids {
                if exclude == Some(pid) { continue; }
                self.send_to(pid, msg.clone());
            }
        }
    }

    fn player_name(&self, id: u32) -> String {
        self.players.get(&id).map(|p| p.name.clone()).unwrap_or_default()
    }

    fn player_info(&self, id: u32) -> Option<PlayerInfo> {
        let p = self.players.get(&id)?;
        let (status, table_id) = if let Some(tid) = p.table_id {
            if let Some(rid) = p.room_id {
                if let Some(room) = self.rooms.get(&rid) {
                    if let Some(table) = room.tables.get(&tid) {
                        if table.white == Some(id) || table.black == Some(id) {
                            (PlayerStatus::Playing, Some(tid))
                        } else {
                            (PlayerStatus::Spectating, Some(tid))
                        }
                    } else { (PlayerStatus::Idle, None) }
                } else { (PlayerStatus::Idle, None) }
            } else { (PlayerStatus::Idle, None) }
        } else { (PlayerStatus::Idle, None) };
        Some(PlayerInfo { id, name: p.name.clone(), status, table_id })
    }

    fn table_info(table: &GameTable, players: &HashMap<u32, ConnectedPlayer>) -> TableInfo {
        let player_ref = |id: u32| -> PlayerRef {
            let name = players.get(&id).map(|p| p.name.clone()).unwrap_or_default();
            PlayerRef { id, name }
        };
        TableInfo {
            id: table.id,
            white: table.white.map(&player_ref),
            black: table.black.map(&player_ref),
            spectator_count: table.spectators.len() as u32,
            has_game: table.game_active,
        }
    }

    fn room_info(room: &Room) -> RoomInfo {
        let active_games = room.tables.values().filter(|t| t.game_active).count() as u32;
        RoomInfo {
            id: room.id,
            name: room.name.clone(),
            player_count: room.player_ids.len() as u32,
            table_count: room.tables.len() as u32,
            active_games,
        }
    }

    fn system_chat(&self, room_id: u32, body: String) {
        self.broadcast_room(room_id, ServerMsg::ChatMessage {
            sender: String::new(), body, kind: ChatKind::System,
        }, None);
    }

    // ── Message Handling ───────────────────────────────────────────

    fn handle_msg(&mut self, pid: u32, msg: ClientMsg) {
        match msg {
            ClientMsg::SetName { name } => {
                if let Some(p) = self.players.get_mut(&pid) { p.name = name; }
            }
            ClientMsg::ListRooms => {
                let rooms: Vec<RoomInfo> = self.rooms.values().map(|r| Self::room_info(r)).collect();
                self.send_to(pid, ServerMsg::RoomList { rooms });
            }
            ClientMsg::CreateRoom { name } => {
                let room_id = self.alloc_id();
                let room = Room {
                    id: room_id,
                    name: name.clone(),
                    moderator: pid,
                    player_ids: vec![pid],
                    tables: HashMap::new(),
                    next_table_id: 1,
                    main_board_mode: BoardMode::Tutorial,
                    main_board_pos: Position::start(),
                };
                let info = Self::room_info(&room);
                self.rooms.insert(room_id, room);
                if let Some(p) = self.players.get_mut(&pid) { p.room_id = Some(room_id); }
                let players = vec![self.player_info(pid).unwrap()];
                self.send_to(pid, ServerMsg::RoomJoined { room: info, players, tables: vec![] });
            }
            ClientMsg::JoinRoom { room_id } => {
                self.leave_room(pid);
                if !self.rooms.contains_key(&room_id) {
                    self.send_to(pid, ServerMsg::Error { msg: "Room not found".into() });
                    return;
                }
                self.rooms.get_mut(&room_id).unwrap().player_ids.push(pid);
                if let Some(p) = self.players.get_mut(&pid) { p.room_id = Some(room_id); }

                let room = self.rooms.get(&room_id).unwrap();
                let info = Self::room_info(room);
                let players: Vec<PlayerInfo> = room.player_ids.iter()
                    .filter_map(|&id| self.player_info(id)).collect();
                let tables: Vec<TableInfo> = room.tables.values()
                    .map(|t| Self::table_info(t, &self.players)).collect();
                self.send_to(pid, ServerMsg::RoomJoined { room: info, players, tables });

                let pinfo = self.player_info(pid).unwrap();
                let name = pinfo.name.clone();
                self.broadcast_room(room_id, ServerMsg::PlayerJoined { player: pinfo }, Some(pid));
                self.system_chat(room_id, format!("{name} joined"));
            }
            ClientMsg::LeaveRoom => { self.leave_room(pid); }

            ClientMsg::CreateTable => {
                let room_id = match self.players.get(&pid).and_then(|p| p.room_id) {
                    Some(r) => r,
                    None => return,
                };
                let room = match self.rooms.get_mut(&room_id) {
                    Some(r) => r,
                    None => return,
                };
                let table_id = room.next_table_id;
                room.next_table_id += 1;
                let table = GameTable {
                    id: table_id,
                    white: Some(pid),
                    black: None,
                    spectators: Vec::new(),
                    position: Position::start(),
                    game_active: false,
                };
                let info = Self::table_info(&table, &self.players);
                room.tables.insert(table_id, table);
                if let Some(p) = self.players.get_mut(&pid) { p.table_id = Some(table_id); }

                self.broadcast_room(room_id, ServerMsg::TableCreated { table: info.clone() }, None);
                let name = self.player_name(pid);
                self.system_chat(room_id, format!("{name} created a game table"));
            }
            ClientMsg::JoinTable { table_id } => {
                let room_id = match self.players.get(&pid).and_then(|p| p.room_id) {
                    Some(r) => r,
                    None => return,
                };
                // Leave current table first
                self.leave_table(pid);

                let room = match self.rooms.get_mut(&room_id) {
                    Some(r) => r,
                    None => return,
                };
                let table = match room.tables.get_mut(&table_id) {
                    Some(t) => t,
                    None => {
                        self.send_to(pid, ServerMsg::Error { msg: "Table not found".into() });
                        return;
                    }
                };

                // Try to sit as player, else spectate
                if table.white.is_none() {
                    table.white = Some(pid);
                } else if table.black.is_none() {
                    table.black = Some(pid);
                } else {
                    table.spectators.push(pid);
                }

                if let Some(p) = self.players.get_mut(&pid) { p.table_id = Some(table_id); }

                // If both seats filled, start game
                let room = self.rooms.get(&room_id).unwrap();
                let table = room.tables.get(&table_id).unwrap();
                let info = Self::table_info(table, &self.players);
                let fen = position_to_fen(&table.position);

                self.send_to(pid, ServerMsg::TableJoined { table: info.clone(), fen: fen.clone() });
                self.broadcast_room(room_id, ServerMsg::TableUpdated { table: info }, Some(pid));

                if table.white.is_some() && table.black.is_some() && !table.game_active {
                    let white = table.white.unwrap();
                    let black = table.black.unwrap();
                    // Start game
                    let room = self.rooms.get_mut(&room_id).unwrap();
                    let table = room.tables.get_mut(&table_id).unwrap();
                    table.game_active = true;
                    table.position = Position::start();
                    let fen = position_to_fen(&table.position);

                    self.broadcast_room(room_id, ServerMsg::GameStarted {
                        table_id, white, black, fen,
                    }, None);
                    let wname = self.player_name(white);
                    let bname = self.player_name(black);
                    self.system_chat(room_id, format!("{wname} vs {bname} — game started!"));
                }
            }
            ClientMsg::LeaveTable => { self.leave_table(pid); }

            ClientMsg::MakeMove { uci } => {
                let (room_id, table_id) = match self.player_location(pid) {
                    Some(loc) => loc,
                    None => return,
                };

                let result = {
                    let room = match self.rooms.get_mut(&room_id) { Some(r) => r, None => return };
                    let table = match room.tables.get_mut(&table_id) { Some(t) => t, None => return };

                    if !table.game_active {
                        Err("No active game")
                    } else {
                        let is_white = table.position.side_to_move == board::Color::White;
                        let correct = (is_white && table.white == Some(pid))
                            || (!is_white && table.black == Some(pid));
                        if !correct {
                            Err("Not your turn")
                        } else if let Some(mv) = Move::from_uci(&uci) {
                            let legal = table.position.legal_moves();
                            if legal.iter().any(|m| m.from == mv.from && m.to == mv.to && m.promotion == mv.promotion) {
                                table.position = table.position.make_move(mv);
                                let fen = position_to_fen(&table.position);
                                let checkmate = table.position.is_checkmate();
                                let stalemate = table.position.is_stalemate();
                                Ok((fen, checkmate, stalemate))
                            } else {
                                Err("Illegal move")
                            }
                        } else {
                            Err("Bad move format")
                        }
                    }
                };

                match result {
                    Ok((fen, checkmate, stalemate)) => {
                        self.broadcast_room(room_id, ServerMsg::MoveMade { table_id, uci, fen }, None);
                        if checkmate {
                            self.broadcast_room(room_id, ServerMsg::GameOver {
                                table_id, reason: "Checkmate".into(), winner: Some(pid),
                            }, None);
                            self.end_game(room_id, table_id);
                        } else if stalemate {
                            self.broadcast_room(room_id, ServerMsg::GameOver {
                                table_id, reason: "Stalemate".into(), winner: None,
                            }, None);
                            self.end_game(room_id, table_id);
                        }
                    }
                    Err(e) => { self.send_to(pid, ServerMsg::Error { msg: e.into() }); }
                }
            }
            ClientMsg::Resign => {
                if let Some((room_id, table_id)) = self.player_location(pid) {
                    let winner = {
                        let room = match self.rooms.get(&room_id) { Some(r) => r, None => return };
                        let table = match room.tables.get(&table_id) { Some(t) => t, None => return };
                        if table.white == Some(pid) { table.black } else { table.white }
                    };
                    let name = self.player_name(pid);
                    self.broadcast_room(room_id, ServerMsg::GameOver {
                        table_id, reason: format!("{name} resigned"), winner,
                    }, None);
                    self.end_game(room_id, table_id);
                }
            }
            ClientMsg::SendChat { body } => {
                let room_id = self.players.get(&pid).and_then(|p| p.room_id);
                if let Some(room_id) = room_id {
                    let name = self.player_name(pid);
                    self.broadcast_room(room_id, ServerMsg::ChatMessage {
                        sender: name, body, kind: ChatKind::Player,
                    }, None);
                }
            }
            ClientMsg::SetMainBoardMode { mode } => {
                let room_id = match self.players.get(&pid).and_then(|p| p.room_id) {
                    Some(r) => r,
                    None => return,
                };
                if let Some(room) = self.rooms.get_mut(&room_id) {
                    if room.moderator == pid {
                        room.main_board_mode = mode.clone();
                        room.main_board_pos = Position::start();
                        let fen = position_to_fen(&room.main_board_pos);
                        self.broadcast_room(room_id, ServerMsg::MainBoardUpdate { mode, fen }, None);
                    }
                }
            }
            ClientMsg::Rematch => {
                if let Some((room_id, table_id)) = self.player_location(pid) {
                    let should_start = if let Some(room) = self.rooms.get(&room_id) {
                        if let Some(table) = room.tables.get(&table_id) {
                            !table.game_active && table.white.is_some() && table.black.is_some()
                        } else { false }
                    } else { false };

                    if should_start {
                        // Swap colors for the rematch
                        let (white, black) = if let Some(room) = self.rooms.get_mut(&room_id) {
                            if let Some(table) = room.tables.get_mut(&table_id) {
                                let old_white = table.white;
                                table.white = table.black;
                                table.black = old_white;
                                table.game_active = true;
                                table.position = Position::start();
                                (table.white.unwrap(), table.black.unwrap())
                            } else { return; }
                        } else { return; };

                        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
                        self.broadcast_room(room_id, ServerMsg::GameStarted {
                            table_id, white, black, fen,
                        }, None);
                        let wname = self.player_name(white);
                        let bname = self.player_name(black);
                        self.system_chat(room_id, format!("Rematch! {wname} (W) vs {bname} (B)"));
                    }
                }
            }
            ClientMsg::MainBoardMove { uci } => {
                // Moderator moves on main board (tutorial mode)
                let room_id = match self.players.get(&pid).and_then(|p| p.room_id) {
                    Some(r) => r,
                    None => return,
                };
                if let Some(room) = self.rooms.get_mut(&room_id) {
                    if room.moderator == pid {
                        if let Some(mv) = Move::from_uci(&uci) {
                            room.main_board_pos = room.main_board_pos.make_move(mv);
                            let fen = position_to_fen(&room.main_board_pos);
                            let mode = room.main_board_mode.clone();
                            self.broadcast_room(room_id, ServerMsg::MainBoardUpdate { mode, fen }, None);
                        }
                    }
                }
            }
        }
    }

    fn player_location(&self, pid: u32) -> Option<(u32, u32)> {
        let p = self.players.get(&pid)?;
        Some((p.room_id?, p.table_id?))
    }

    fn end_game(&mut self, room_id: u32, table_id: u32) {
        if let Some(room) = self.rooms.get_mut(&room_id) {
            if let Some(table) = room.tables.get_mut(&table_id) {
                table.game_active = false;
                table.position = Position::start();
            }
            // Broadcast updated table info
            if let Some(table) = room.tables.get(&table_id) {
                let info = Self::table_info(table, &self.players);
                self.broadcast_room(room_id, ServerMsg::TableUpdated { table: info }, None);
            }
        }
    }

    fn leave_table(&mut self, pid: u32) {
        let (room_id, table_id) = match self.player_location(pid) {
            Some(loc) => loc,
            None => return,
        };

        if let Some(p) = self.players.get_mut(&pid) { p.table_id = None; }

        // Collect info before broadcasting
        let player_name = self.player_name(pid);
        let (should_remove, game_ended, winner) = if let Some(room) = self.rooms.get_mut(&room_id) {
            if let Some(table) = room.tables.get_mut(&table_id) {
                table.spectators.retain(|&id| id != pid);
                if table.white == Some(pid) { table.white = None; }
                if table.black == Some(pid) { table.black = None; }

                let (ended, win) = if table.game_active && (table.white.is_none() || table.black.is_none()) {
                    table.game_active = false;
                    (true, table.white.or(table.black))
                } else { (false, None) };

                let empty = table.white.is_none() && table.black.is_none() && table.spectators.is_empty();
                (empty, ended, win)
            } else { (false, false, None) }
        } else { (false, false, None) };

        if game_ended {
            self.broadcast_room(room_id, ServerMsg::GameOver {
                table_id,
                reason: format!("{player_name} left the table"),
                winner,
            }, None);
        }

        if should_remove {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                room.tables.remove(&table_id);
            }
            self.broadcast_room(room_id, ServerMsg::TableRemoved { table_id }, None);
        } else {
            // Update table info
            if let Some(room) = self.rooms.get(&room_id) {
                if let Some(table) = room.tables.get(&table_id) {
                    let info = Self::table_info(table, &self.players);
                    self.broadcast_room(room_id, ServerMsg::TableUpdated { table: info }, None);
                }
            }
        }
    }

    fn leave_room(&mut self, pid: u32) {
        self.leave_table(pid);
        let room_id = self.players.get(&pid).and_then(|p| p.room_id);
        if let Some(room_id) = room_id {
            let name = self.player_name(pid);
            if let Some(room) = self.rooms.get_mut(&room_id) {
                room.player_ids.retain(|&id| id != pid);
            }
            if let Some(p) = self.players.get_mut(&pid) { p.room_id = None; }
            self.broadcast_room(room_id, ServerMsg::PlayerLeft { player_id: pid }, None);
            self.system_chat(room_id, format!("{name} left"));
            // Remove empty rooms
            if self.rooms.get(&room_id).map_or(false, |r| r.player_ids.is_empty()) {
                self.rooms.remove(&room_id);
            }
        }
    }

    fn disconnect(&mut self, pid: u32) {
        self.leave_room(pid);
        self.players.remove(&pid);
    }
}

fn position_to_fen(pos: &Position) -> String {
    let mut fen = String::new();
    for rank in (0..8).rev() {
        let mut empty = 0u32;
        for file in 0..8 {
            let sq = rank * 8 + file;
            match pos.piece_at(sq) {
                Some((pt, color)) => {
                    if empty > 0 { fen.push(char::from_digit(empty, 10).unwrap()); empty = 0; }
                    let c = match (pt, color) {
                        (board::PAWN, board::Color::White) => 'P', (board::KNIGHT, board::Color::White) => 'N',
                        (board::BISHOP, board::Color::White) => 'B', (board::ROOK, board::Color::White) => 'R',
                        (board::QUEEN, board::Color::White) => 'Q', (board::KING, board::Color::White) => 'K',
                        (board::PAWN, board::Color::Black) => 'p', (board::KNIGHT, board::Color::Black) => 'n',
                        (board::BISHOP, board::Color::Black) => 'b', (board::ROOK, board::Color::Black) => 'r',
                        (board::QUEEN, board::Color::Black) => 'q', (board::KING, board::Color::Black) => 'k',
                        _ => '?',
                    };
                    fen.push(c);
                }
                None => { empty += 1; }
            }
        }
        if empty > 0 { fen.push(char::from_digit(empty, 10).unwrap()); }
        if rank > 0 { fen.push('/'); }
    }
    fen.push(' ');
    fen.push(if pos.side_to_move == board::Color::White { 'w' } else { 'b' });
    fen.push_str(" KQkq - 0 1");
    fen
}

// ── Public API ─────────────────────────────────────────────────────

type State = Arc<Mutex<ServerState>>;

/// Start the server on a background thread on DEFAULT_PORT. Returns immediately.
pub fn start_server() {
    start_server_on(DEFAULT_PORT);
}

/// Start the server on a background thread on CENTRAL_SERVER_PORT. Returns immediately.
pub fn start_central_server() {
    start_server_on(crate::protocol::CENTRAL_SERVER_PORT);
}

pub fn start_server_on(port: u16) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create server runtime");
        rt.block_on(run_server(port));
    });
}

async fn run_server(port: u16) {
    let addr = format!("0.0.0.0:{port}");
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(_) => return, // Port in use — another instance is hosting
    };

    let state: State = Arc::new(Mutex::new(ServerState::new()));

    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };

        let state = state.clone();
        tokio::spawn(async move {
            let ws = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(_) => return,
            };

            let (mut ws_tx, mut ws_rx) = ws.split();
            let (tx, mut rx) = mpsc::unbounded_channel::<ServerMsg>();

            let pid = {
                let mut s = state.lock().await;
                let id = s.alloc_id();
                s.players.insert(id, ConnectedPlayer {
                    id, name: format!("Player{id}"), room_id: None, table_id: None,
                    tx: tx.clone(),
                });
                let _ = tx.send(ServerMsg::Welcome { your_id: id });
                id
            };

            let write_task = tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let json = serde_json::to_string(&msg).unwrap();
                    if ws_tx.send(Message::Text(json)).await.is_err() { break; }
                }
            });

            while let Some(Ok(msg)) = ws_rx.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        let mut s = state.lock().await;
                        s.handle_msg(pid, client_msg);
                    }
                }
            }

            { let mut s = state.lock().await; s.disconnect(pid); }
            write_task.abort();
        });
    }
}
