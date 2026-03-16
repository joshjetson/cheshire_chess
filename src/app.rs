use std::path::Path;
use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::audio::Audio;
use crate::board::{Move, Position};
use crate::canvas::{CanvasMode, CanvasState, CustomPieces, PIECE_TYPES, SHAPE_PALETTE};
use crate::net::NetClient;
use crate::protocol::*;
use crate::puzzle::{Puzzle, PuzzleIndex, TACTIC_THEMES};
use crate::settings::{Settings, SETTINGS_ITEMS, SOUND_EVENT_NAMES, SYNTH_PARAM_NAMES};
use crate::tracker::{self, RemoteServer};

pub enum Screen {
    Menu,
    ThemePicker,
    Puzzle,
    Analysis,
    Results,
    Canvas,
    RoomBrowser,
    RoomLobby,
    LiveGame,
    Settings,
    SoundSettings,
    SoundEventEdit,
    NameEdit,
}

pub struct ChatState {
    pub messages: Vec<(String, String, ChatKind)>, // (sender, body, kind)
    pub input: String,
    pub typing: bool, // is the user typing in chat?
}

impl ChatState {
    fn new() -> Self {
        Self { messages: Vec::new(), input: String::new(), typing: false }
    }
}

pub struct App {
    pub screen: Screen,
    pub board: Position,
    pub cursor: u8,
    pub running: bool,
    pub message: String,
    pub menu_selection: usize,
    pub theme_selection: usize,
    // Puzzles
    pub puzzle_index: Option<PuzzleIndex>,
    pub puzzle_queue: Vec<Puzzle>,
    pub puzzle_pos: usize,
    pub puzzle_move_index: usize,
    pub selected_sq: Option<u8>,
    pub highlights: Vec<u8>,
    pub score_correct: usize,
    pub score_total: usize,
    // Canvas
    pub custom_pieces: CustomPieces,
    pub canvas: CanvasState,
    // Online
    pub net: Option<NetClient>,
    pub net_rx: Option<mpsc::Receiver<ServerMsg>>,
    pub my_id: Option<u32>,
    pub player_name: String,
    pub room_list: Vec<RoomInfo>,
    pub room_selection: usize,
    pub current_room: Option<RoomInfo>,
    pub room_players: Vec<PlayerInfo>,
    pub player_selection: usize,
    pub chat: ChatState,
    // Tables
    pub tables: Vec<TableInfo>,
    pub table_selection: usize,
    pub current_table: Option<u32>,
    pub live_white: Option<u32>,
    pub live_black: Option<u32>,
    pub game_active: bool,
    // Discovery
    pub remote_servers: Vec<RemoteServer>,
    pub heartbeat_tx: Option<mpsc::Sender<u32>>,
    pub public_ip: Option<String>,
    // Audio & Settings
    pub audio: Option<Audio>,
    pub settings: Settings,
    pub settings_selection: usize,
    pub sound_event_selection: usize,
    pub sound_param_selection: usize,
    pub name_input: String,
}

const MENU_ITEMS: &[&str] = &[
    "Practice Tactics",
    "View Starting Position",
    "Go Online",
    "Settings",
    "Quit",
];

const PUZZLE_BATCH_SIZE: usize = 200;

impl App {
    pub fn new(data_dir: &Path) -> Self {
        let pieces_path = data_dir.join("custom_pieces.txt");
        let settings = Settings::load(data_dir);
        let player_name = settings.player_name.clone();
        Self {
            screen: Screen::Menu,
            board: Position::start(),
            cursor: 28,
            running: true,
            message: String::from("hjkl/arrows to navigate, Enter to select"),
            menu_selection: 0,
            theme_selection: 0,
            puzzle_index: None,
            puzzle_queue: Vec::new(),
            puzzle_pos: 0,
            puzzle_move_index: 0,
            selected_sq: None,
            highlights: Vec::new(),
            score_correct: 0,
            score_total: 0,
            custom_pieces: CustomPieces::new(pieces_path),
            canvas: CanvasState::new(),
            net: None,
            net_rx: None,
            my_id: None,
            player_name,
            room_list: Vec::new(),
            room_selection: 0,
            current_room: None,
            room_players: Vec::new(),
            player_selection: 0,
            chat: ChatState::new(),
            tables: Vec::new(),
            table_selection: 0,
            current_table: None,
            live_white: None,
            live_black: None,
            game_active: false,
            remote_servers: Vec::new(),
            heartbeat_tx: None,
            public_ip: None,
            audio: Audio::new(),
            settings,
            settings_selection: 0,
            sound_event_selection: 0,
            sound_param_selection: 0,
            name_input: String::new(),
        }
    }

    pub fn build_index(&mut self, path: &Path) -> Result<usize, std::io::Error> {
        let index = PuzzleIndex::build(path)?;
        let total = index.total;
        self.puzzle_index = Some(index);
        Ok(total)
    }

    pub fn menu_items(&self) -> &[&str] { MENU_ITEMS }

    pub fn theme_counts(&self) -> &[(String, String, usize)] {
        match &self.puzzle_index {
            Some(idx) => &idx.theme_counts,
            None => &[],
        }
    }

    pub fn total_puzzles(&self) -> usize {
        self.puzzle_index.as_ref().map_or(0, |idx| idx.total)
    }

    fn send_net(&self, msg: ClientMsg) {
        if let Some(ref net) = self.net {
            net.send(msg);
        }
    }

    pub fn play_sound(&self, f: impl FnOnce(&Audio, &crate::settings::SoundSettings)) {
        if let Some(ref audio) = self.audio {
            f(audio, &self.settings.sound);
        }
    }

    /// Called each tick from the event loop to drain network messages.
    pub fn poll_network(&mut self) {
        let msgs: Vec<ServerMsg> = if let Some(ref rx) = self.net_rx {
            let mut v = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                v.push(msg);
            }
            v
        } else {
            return;
        };

        for msg in msgs {
            self.handle_server_msg(msg);
        }
    }

    fn handle_server_msg(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::Welcome { your_id } => {
                self.my_id = Some(your_id);
                self.send_net(ClientMsg::SetName { name: self.player_name.clone() });
                self.send_net(ClientMsg::ListRooms);
                self.screen = Screen::RoomBrowser;
                self.message = format!("Connected as {}. Browse rooms or create one.", self.player_name);
            }
            ServerMsg::Error { msg: err } => {
                self.message = format!("Server: {err}");
            }
            ServerMsg::RoomList { rooms } => {
                self.room_list = rooms;
                self.room_selection = 0;
            }
            ServerMsg::RoomJoined { room, players, tables } => {
                self.current_room = Some(room);
                self.room_players = players;
                self.tables = tables;
                self.player_selection = 0;
                self.table_selection = 0;
                self.chat = ChatState::new();
                self.screen = Screen::RoomLobby;
                self.message = String::from("[t] new table, Enter=join table, Tab=chat, Esc=leave");
            }
            ServerMsg::PlayerJoined { player } => {
                self.room_players.push(player);
            }
            ServerMsg::PlayerLeft { player_id } => {
                self.room_players.retain(|p| p.id != player_id);
            }
            ServerMsg::TableCreated { table } => {
                self.tables.push(table);
            }
            ServerMsg::TableUpdated { table } => {
                if let Some(existing) = self.tables.iter_mut().find(|t| t.id == table.id) {
                    *existing = table;
                }
            }
            ServerMsg::TableRemoved { table_id } => {
                self.tables.retain(|t| t.id != table_id);
            }
            ServerMsg::TableJoined { table, fen } => {
                self.current_table = Some(table.id);
                if let Some(pos) = Position::from_fen(&fen) {
                    self.board = pos;
                }
                self.screen = Screen::LiveGame;
                self.message = String::from("At the table. Waiting for game...");
            }
            ServerMsg::GameStarted { table_id, white, black, fen } => {
                self.live_white = Some(white);
                self.live_black = Some(black);
                self.game_active = true;
                if let Some(pos) = Position::from_fen(&fen) {
                    self.board = pos;
                }
                self.selected_sq = None;
                self.highlights.clear();
                self.cursor = 28;
                if self.current_table != Some(table_id) {
                    // We're watching from the lobby
                    return;
                }
                self.screen = Screen::LiveGame;
                let my_color = if Some(white) == self.my_id {
                    "White"
                } else if Some(black) == self.my_id {
                    "Black"
                } else {
                    "Spectator"
                };
                self.message = format!("Game started! You are {my_color}.");
            }
            ServerMsg::MoveMade { table_id, uci: _, fen } => {
                if self.current_table == Some(table_id) {
                    if let Some(pos) = Position::from_fen(&fen) {
                        let is_check = pos.in_check(pos.side_to_move);
                        if is_check {
                            self.play_sound(|a, s| a.play_check(s));
                        } else {
                            self.play_sound(|a, s| a.play_move(s));
                        }
                        self.board = pos;
                    }
                    self.selected_sq = None;
                    self.highlights.clear();
                    if self.is_my_turn() {
                        self.message = String::from("Your move!");
                    } else {
                        self.message = String::from("Opponent's turn...");
                    }
                }
            }
            ServerMsg::GameOver { table_id, reason, winner } => {
                if self.current_table == Some(table_id) {
                    let result = match winner {
                        Some(id) if Some(id) == self.my_id => "You win!",
                        Some(_) => "You lose.",
                        None => "Draw.",
                    };
                    self.play_sound(|a, s| a.play_checkmate(s));
                    self.message = format!("{reason} — {result} Press Esc to return.");
                    self.game_active = false;
                }
            }
            ServerMsg::MainBoardUpdate { mode: _, fen } => {
                // Could show on lobby screen if desired
                let _ = fen;
            }
            ServerMsg::ChatMessage { sender, body, kind } => {
                self.chat.messages.push((sender, body, kind));
            }
        }
    }

    fn is_my_turn(&self) -> bool {
        let my_id = match self.my_id { Some(id) => id, None => return false };
        match self.board.side_to_move {
            crate::board::Color::White => self.live_white == Some(my_id),
            crate::board::Color::Black => self.live_black == Some(my_id),
        }
    }

    #[allow(dead_code)]
    fn am_playing(&self) -> bool {
        self.game_active && (self.live_white == self.my_id || self.live_black == self.my_id)
    }

    // ── Key Handling ───────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => {
                    self.running = false;
                    return;
                }
                _ => {}
            }
        }

        match self.screen {
            Screen::Menu => self.handle_menu_key(key),
            Screen::ThemePicker => self.handle_theme_picker_key(key),
            Screen::Puzzle => self.handle_puzzle_key(key),
            Screen::Analysis => self.handle_analysis_key(key),
            Screen::Results => self.handle_results_key(key),
            Screen::Canvas => self.handle_canvas_key(key),
            Screen::Settings => self.handle_settings_key(key),
            Screen::SoundSettings => self.handle_sound_settings_key(key),
            Screen::SoundEventEdit => self.handle_sound_event_edit_key(key),
            Screen::NameEdit => self.handle_name_edit_key(key),
            Screen::RoomBrowser => self.handle_room_browser_key(key),
            Screen::RoomLobby => self.handle_room_lobby_key(key),
            Screen::LiveGame => self.handle_live_game_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => { self.running = false; }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.menu_selection > 0 { self.menu_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.menu_selection < MENU_ITEMS.len() - 1 { self.menu_selection += 1; }
            }
            KeyCode::Enter | KeyCode::Char('l') => match self.menu_selection {
                0 => {
                    if self.puzzle_index.is_none() {
                        self.message = String::from("No puzzles loaded. Place lichess_puzzles.csv in data/");
                    } else {
                        self.screen = Screen::ThemePicker;
                        self.theme_selection = 0;
                        self.message = String::from("Pick a tactic theme");
                    }
                }
                1 => {
                    self.screen = Screen::Analysis;
                    self.board = Position::start();
                    self.message = String::from("hjkl/arrows to move cursor. Esc for menu.");
                }
                2 => {
                    // Go Online — start embedded server, register with tracker, connect
                    if self.net.is_some() {
                        self.send_net(ClientMsg::ListRooms);
                        self.remote_servers = tracker::fetch_servers();
                        self.screen = Screen::RoomBrowser;
                        self.message = String::from("Browse rooms or create one.");
                    } else {
                        crate::server::start_server();
                        std::thread::sleep(std::time::Duration::from_millis(100));

                        // Get public IP and register with tracker
                        let ip = tracker::get_public_ip();
                        self.public_ip = Some(ip.clone());
                        let name = format!("{}'s server", self.player_name);
                        self.heartbeat_tx = Some(tracker::start_heartbeat(
                            ip, crate::protocol::DEFAULT_PORT, name,
                        ));

                        // Fetch other online servers
                        self.remote_servers = tracker::fetch_servers();

                        match NetClient::connect("127.0.0.1") {
                            Ok((client, rx)) => {
                                self.net = Some(client);
                                self.net_rx = Some(rx);
                            }
                            Err(e) => {
                                self.message = format!("Connection failed: {e}");
                            }
                        }
                    }
                }
                3 => {
                    self.settings_selection = 0;
                    self.screen = Screen::Settings;
                    self.message = String::from("Settings");
                }
                4 => { self.running = false; }
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_theme_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.theme_selection > 0 { self.theme_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.theme_selection < TACTIC_THEMES.len() - 1 { self.theme_selection += 1; }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                let (theme_tag, theme_name) = TACTIC_THEMES[self.theme_selection];
                if let Some(ref index) = self.puzzle_index {
                    match index.load_theme(theme_tag, Some(2000), PUZZLE_BATCH_SIZE) {
                        Ok(puzzles) => {
                            if puzzles.is_empty() {
                                self.message = format!("No puzzles found for '{theme_name}'");
                            } else {
                                self.puzzle_queue = puzzles;
                                self.puzzle_pos = 0;
                                self.score_correct = 0;
                                self.score_total = 0;
                                self.load_current_puzzle();
                                self.screen = Screen::Puzzle;
                            }
                        }
                        Err(e) => { self.message = format!("Error loading puzzles: {e}"); }
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                self.screen = Screen::Menu;
                self.message = String::from("hjkl/arrows to navigate, Enter to select");
            }
            _ => {}
        }
    }

    fn load_current_puzzle(&mut self) {
        if let Some(puzzle) = self.puzzle_queue.get(self.puzzle_pos).cloned() {
            if let Some(mut pos) = Position::from_fen(&puzzle.fen) {
                if !puzzle.moves.is_empty() {
                    if let Some(setup_mv) = Move::from_uci(&puzzle.moves[0]) {
                        pos = pos.make_move(setup_mv);
                    }
                }
                self.board = pos;
                self.cursor = 28;
                self.selected_sq = None;
                self.highlights.clear();
                self.puzzle_move_index = 1;
                let total = self.puzzle_queue.len();
                let num = self.puzzle_pos + 1;
                let rating = puzzle.rating;
                let color = match self.board.side_to_move {
                    crate::board::Color::White => "White",
                    crate::board::Color::Black => "Black",
                };
                self.message = format!(
                    "Puzzle {num}/{total} (rating: {rating}) — Play as {color}. Select a piece."
                );
            }
        }
    }

    fn move_cursor(&mut self, key: KeyEvent) {
        let file = self.cursor % 8;
        let rank = self.cursor / 8;
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => { if file > 0 { self.cursor -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if file < 7 { self.cursor += 1; } }
            KeyCode::Up | KeyCode::Char('k') => { if rank < 7 { self.cursor += 8; } }
            KeyCode::Down | KeyCode::Char('j') => { if rank > 0 { self.cursor -= 8; } }
            _ => {}
        }
    }

    fn handle_puzzle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            | KeyCode::Char('h') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('l') => {
                self.move_cursor(key);
            }
            KeyCode::Enter => { self.handle_puzzle_select(); }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.score_total += 1;
                self.advance_puzzle();
            }
            KeyCode::Char('H') => {
                if let Some(puzzle) = self.puzzle_queue.get(self.puzzle_pos) {
                    if self.puzzle_move_index < puzzle.moves.len() {
                        let hint_move = &puzzle.moves[self.puzzle_move_index];
                        self.message = format!("Hint: {hint_move}");
                        if let Some(mv) = Move::from_uci(hint_move) {
                            self.highlights = vec![mv.from, mv.to];
                        }
                        self.play_sound(|a, s| a.play_hint(s));
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.selected_sq = None;
                self.highlights.clear();
                self.screen = Screen::ThemePicker;
                self.message = String::from("Pick a tactic theme");
            }
            _ => {}
        }
    }

    fn handle_puzzle_select(&mut self) {
        let sq = self.cursor;
        if let Some(from) = self.selected_sq {
            if from == sq {
                self.selected_sq = None;
                self.highlights.clear();
                self.message = String::from("Deselected. Pick a piece.");
                return;
            }
            let legal_moves = self.board.legal_moves();
            let mv = legal_moves.iter().find(|m| {
                m.from == from && m.to == sq
                    && (m.promotion.is_none() || m.promotion == Some(crate::board::QUEEN))
            });
            if let Some(&mv) = mv {
                let is_correct = if let Some(puzzle) = self.puzzle_queue.get(self.puzzle_pos) {
                    if self.puzzle_move_index < puzzle.moves.len() {
                        mv.to_uci() == puzzle.moves[self.puzzle_move_index]
                    } else { false }
                } else { false };

                if is_correct {
                    // Check if this is a capture
                    let is_capture = self.board.piece_at(mv.to).is_some();
                    self.board = self.board.make_move(mv);
                    let is_check = self.board.in_check(self.board.side_to_move);
                    let is_checkmate = self.board.is_checkmate();

                    self.selected_sq = None;
                    self.highlights.clear();
                    self.puzzle_move_index += 1;
                    let puzzle_len = self.puzzle_queue.get(self.puzzle_pos).map_or(0, |p| p.moves.len());

                    if self.puzzle_move_index >= puzzle_len {
                        self.score_correct += 1;
                        self.score_total += 1;
                        if is_checkmate {
                            self.play_sound(|a, s| a.play_checkmate(s));
                        } else {
                            self.play_sound(|a, s| a.play_session_complete(s));
                        }
                        self.message = String::from("Correct! Puzzle solved! Press [n] for next.");
                    } else {
                        if is_checkmate {
                            self.play_sound(|a, s| a.play_checkmate(s));
                        } else if is_check {
                            self.play_sound(|a, s| a.play_check(s));
                        } else if is_capture {
                            self.play_sound(|a, s| a.play_capture(s));
                        } else {
                            self.play_sound(|a, s| a.play_correct(s));
                        }
                        if let Some(puzzle) = self.puzzle_queue.get(self.puzzle_pos) {
                            if self.puzzle_move_index < puzzle.moves.len() {
                                let opp_uci = puzzle.moves[self.puzzle_move_index].clone();
                                if let Some(opp_mv) = Move::from_uci(&opp_uci) {
                                    self.board = self.board.make_move(opp_mv);
                                    self.puzzle_move_index += 1;
                                }
                            }
                        }
                        self.message = String::from("Correct! Keep going...");
                    }
                } else {
                    self.play_sound(|a, s| a.play_wrong(s));
                    self.selected_sq = None;
                    self.highlights.clear();
                    self.message = String::from("Wrong move. Try again or [H] for hint.");
                }
            } else {
                let us_color = self.board.side_to_move;
                if let Some((_, color)) = self.board.piece_at(sq) {
                    if color == us_color { self.select_piece(sq); return; }
                }
                self.message = String::from("Illegal move. Try again.");
            }
        } else {
            let us_color = self.board.side_to_move;
            if let Some((_, color)) = self.board.piece_at(sq) {
                if color == us_color { self.select_piece(sq); }
            }
        }
    }

    fn select_piece(&mut self, sq: u8) {
        self.selected_sq = Some(sq);
        let legal = self.board.legal_moves();
        self.highlights = legal.iter().filter(|m| m.from == sq).map(|m| m.to).collect();
        let n = self.highlights.len();
        self.message = format!("{n} legal moves. Move cursor and Enter.");
    }

    fn advance_puzzle(&mut self) {
        self.puzzle_pos += 1;
        self.selected_sq = None;
        self.highlights.clear();
        if self.puzzle_pos >= self.puzzle_queue.len() {
            self.screen = Screen::Results;
            self.message = format!("Session complete! {}/{} correct.", self.score_correct, self.score_total);
        } else {
            self.load_current_puzzle();
        }
    }

    fn handle_analysis_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            | KeyCode::Char('h') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('l') => {
                self.move_cursor(key);
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Menu;
                self.message = String::from("hjkl/arrows to navigate, Enter to select");
            }
            _ => {}
        }
    }

    fn handle_results_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.screen = Screen::Menu;
                self.message = String::from("hjkl/arrows to navigate, Enter to select");
            }
            _ => {}
        }
    }

    // ── Room Browser ───────────────────────────────────────────────

    fn handle_room_browser_key(&mut self, key: KeyEvent) {
        let total_items = self.room_list.len() + self.remote_servers.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.room_selection > 0 { self.room_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total_items > 0 && self.room_selection < total_items - 1 {
                    self.room_selection += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                let local_count = self.room_list.len();
                if self.room_selection < local_count {
                    // Join local room
                    if let Some(room) = self.room_list.get(self.room_selection) {
                        self.send_net(ClientMsg::JoinRoom { room_id: room.id });
                    }
                } else {
                    // Connect to remote server
                    let remote_idx = self.room_selection - local_count;
                    if let Some(server) = self.remote_servers.get(remote_idx).cloned() {
                        // Disconnect from local, connect to remote
                        self.net = None;
                        self.net_rx = None;
                        self.my_id = None;
                        match NetClient::connect(&server.host) {
                            Ok((client, rx)) => {
                                self.net = Some(client);
                                self.net_rx = Some(rx);
                                self.message = format!("Connecting to {}...", server.name);
                            }
                            Err(e) => {
                                self.message = format!("Failed to connect: {e}");
                                // Reconnect to local
                                if let Ok((client, rx)) = NetClient::connect("127.0.0.1") {
                                    self.net = Some(client);
                                    self.net_rx = Some(rx);
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let name = format!("{}'s room", self.player_name);
                self.send_net(ClientMsg::CreateRoom { name });
            }
            KeyCode::Char('r') => {
                self.send_net(ClientMsg::ListRooms);
                self.remote_servers = tracker::fetch_servers();
                // Filter out our own server from remote list
                if let Some(ref ip) = self.public_ip {
                    self.remote_servers.retain(|s| s.host != *ip);
                }
                self.message = String::from("Refreshed.");
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Menu;
                self.message = String::from("hjkl/arrows to navigate, Enter to select");
            }
            _ => {}
        }
    }

    // ── Room Lobby ─────────────────────────────────────────────────

    fn handle_room_lobby_key(&mut self, key: KeyEvent) {
        if self.chat.typing {
            match key.code {
                KeyCode::Enter => {
                    if !self.chat.input.is_empty() {
                        let body = self.chat.input.clone();
                        self.chat.input.clear();
                        self.send_net(ClientMsg::SendChat { body });
                    }
                }
                KeyCode::Char(c) => { self.chat.input.push(c); }
                KeyCode::Backspace => { self.chat.input.pop(); }
                KeyCode::Esc => {
                    self.chat.typing = false;
                    self.message = String::from("[t] new table, Enter=join table, Tab=chat, Esc=leave");
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.chat.typing = true;
                self.message = String::from("Type message, Enter to send, Esc to stop");
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.table_selection > 0 { self.table_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.tables.is_empty() && self.table_selection < self.tables.len() - 1 {
                    self.table_selection += 1;
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.send_net(ClientMsg::CreateTable);
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                if let Some(table) = self.tables.get(self.table_selection) {
                    self.send_net(ClientMsg::JoinTable { table_id: table.id });
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.send_net(ClientMsg::LeaveRoom);
                self.current_room = None;
                self.room_players.clear();
                self.tables.clear();
                self.send_net(ClientMsg::ListRooms);
                self.screen = Screen::RoomBrowser;
                self.message = String::from("Browse rooms or create one.");
            }
            _ => {}
        }
    }

    // ── Live Game ──────────────────────────────────────────────────

    fn handle_live_game_key(&mut self, key: KeyEvent) {
        // Chat in live game
        if self.chat.typing {
            match key.code {
                KeyCode::Enter => {
                    if !self.chat.input.is_empty() {
                        let body = self.chat.input.clone();
                        self.chat.input.clear();
                        self.send_net(ClientMsg::SendChat { body });
                    }
                }
                KeyCode::Char(c) => { self.chat.input.push(c); }
                KeyCode::Backspace => { self.chat.input.pop(); }
                KeyCode::Esc => { self.chat.typing = false; }
                _ => {}
            }
            return;
        }

        // Game is over — Esc to go back
        if !self.game_active {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.send_net(ClientMsg::LeaveTable);
                    self.current_table = None;
                    self.screen = Screen::RoomLobby;
                    self.message = String::from("[t] new table, Enter=join table, Tab=chat, Esc=leave");
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            | KeyCode::Char('h') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('l') => {
                if self.is_my_turn() { self.move_cursor(key); }
            }
            KeyCode::Enter => {
                if self.is_my_turn() { self.handle_live_game_select(); }
            }
            KeyCode::Tab => {
                self.chat.typing = true;
                self.message = String::from("Type message, Enter to send, Esc to stop");
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Resign
                self.send_net(ClientMsg::Resign);
            }
            KeyCode::Esc => {
                self.send_net(ClientMsg::LeaveTable);
                self.current_table = None;
                self.game_active = false;
                self.screen = Screen::RoomLobby;
                self.message = String::from("[t] new table, Enter=join table, Tab=chat, Esc=leave");
            }
            _ => {}
        }
    }

    fn handle_live_game_select(&mut self) {
        let sq = self.cursor;
        if let Some(from) = self.selected_sq {
            if from == sq {
                self.selected_sq = None;
                self.highlights.clear();
                return;
            }
            let legal_moves = self.board.legal_moves();
            let mv = legal_moves.iter().find(|m| {
                m.from == from && m.to == sq
                    && (m.promotion.is_none() || m.promotion == Some(crate::board::QUEEN))
            });
            if let Some(&mv) = mv {
                self.send_net(ClientMsg::MakeMove { uci: mv.to_uci() });
                self.selected_sq = None;
                self.highlights.clear();
                self.message = String::from("Move sent...");
            } else {
                let us_color = self.board.side_to_move;
                if let Some((_, color)) = self.board.piece_at(sq) {
                    if color == us_color { self.select_piece(sq); return; }
                }
            }
        } else {
            let us_color = self.board.side_to_move;
            if let Some((_, color)) = self.board.piece_at(sq) {
                if color == us_color { self.select_piece(sq); }
            }
        }
    }

    // ── Settings ───────────────────────────────────────────────────

    fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.settings_selection > 0 { self.settings_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.settings_selection < SETTINGS_ITEMS.len() - 1 { self.settings_selection += 1; }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                match self.settings_selection {
                    0 => {
                        // Player Name
                        self.name_input = self.settings.player_name.clone();
                        self.screen = Screen::NameEdit;
                        self.message = String::from("Type your name, Enter to save, Esc to cancel");
                    }
                    1 => {
                        // Sound Settings
                        self.sound_event_selection = 0;
                        self.screen = Screen::SoundSettings;
                        self.message = String::from("Select a sound event to edit");
                    }
                    2 => {
                        // Piece Canvas
                        self.canvas = CanvasState::new();
                        self.screen = Screen::Canvas;
                        self.message = String::from("Select a piece to draw");
                    }
                    3 => {
                        // Back
                        self.screen = Screen::Menu;
                        self.message = String::from("hjkl/arrows to navigate, Enter to select");
                    }
                    _ => {}
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Menu;
                self.message = String::from("hjkl/arrows to navigate, Enter to select");
            }
            _ => {}
        }
    }

    fn handle_name_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.name_input.is_empty() {
                    self.settings.player_name = self.name_input.clone();
                    self.player_name = self.name_input.clone();
                    let _ = self.settings.save();
                    self.message = format!("Name set to: {}", self.player_name);
                }
                self.screen = Screen::Settings;
            }
            KeyCode::Char(c) => { self.name_input.push(c); }
            KeyCode::Backspace => { self.name_input.pop(); }
            KeyCode::Esc => {
                self.screen = Screen::Settings;
                self.message = String::from("Settings");
            }
            _ => {}
        }
    }

    fn handle_sound_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sound_event_selection > 0 { self.sound_event_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.sound_event_selection < SOUND_EVENT_NAMES.len() - 1 {
                    self.sound_event_selection += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                self.sound_param_selection = 0;
                self.screen = Screen::SoundEventEdit;
                self.message = format!("Editing: {} — hjkl to adjust, [p] preview, Esc back",
                    SOUND_EVENT_NAMES[self.sound_event_selection]);
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // Toggle mute
                self.settings.sound.enabled = !self.settings.sound.enabled;
                let state = if self.settings.sound.enabled { "ON" } else { "OFF" };
                self.message = format!("Sound: {state}");
                let _ = self.settings.save();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Settings;
                self.message = String::from("Settings");
            }
            _ => {}
        }
    }

    pub fn get_event_params_mut(&mut self, idx: usize) -> &mut crate::settings::SynthParams {
        let e = &mut self.settings.sound.events;
        match idx {
            0 => &mut e.login,
            1 => &mut e.exit,
            2 => &mut e.piece_move,
            3 => &mut e.capture,
            4 => &mut e.check,
            5 => &mut e.checkmate,
            6 => &mut e.wrong_move,
            7 => &mut e.correct,
            8 => &mut e.hint,
            9 => &mut e.tick,
            _ => &mut e.select,
        }
    }

    pub fn get_event_params(&self, idx: usize) -> &crate::settings::SynthParams {
        let e = &self.settings.sound.events;
        match idx {
            0 => &e.login,
            1 => &e.exit,
            2 => &e.piece_move,
            3 => &e.capture,
            4 => &e.check,
            5 => &e.checkmate,
            6 => &e.wrong_move,
            7 => &e.correct,
            8 => &e.hint,
            9 => &e.tick,
            _ => &e.select,
        }
    }

    fn handle_sound_event_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sound_param_selection > 0 { self.sound_param_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.sound_param_selection < SYNTH_PARAM_NAMES.len() - 1 {
                    self.sound_param_selection += 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.adjust_param(1);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.adjust_param(-1);
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Preview
                let params = self.get_event_params(self.sound_event_selection).clone();
                self.play_sound(|a, s| a.play(&params, s));
            }
            KeyCode::Char('s') => {
                let _ = self.settings.save();
                self.message = String::from("Sound settings saved!");
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                let _ = self.settings.save();
                self.screen = Screen::SoundSettings;
                self.message = String::from("Select a sound event to edit");
            }
            _ => {}
        }
    }

    fn adjust_param(&mut self, dir: i8) {
        let param_sel = self.sound_param_selection;
        let params = self.get_event_params_mut(self.sound_event_selection);
        match param_sel {
            0 => { params.waveform = params.waveform.next(); } // Waveform
            1 => { params.frequency = (params.frequency + dir as f32 * 10.0).clamp(20.0, 4000.0); }
            2 => { params.attack = (params.attack + dir as f32 * 0.01).clamp(0.001, 1.0); }
            3 => { params.decay = (params.decay + dir as f32 * 0.01).clamp(0.01, 1.0); }
            4 => { params.sustain = (params.sustain + dir as f32 * 0.05).clamp(0.0, 1.0); }
            5 => { params.release = (params.release + dir as f32 * 0.01).clamp(0.01, 2.0); }
            6 => { params.volume = (params.volume + dir as f32 * 0.02).clamp(0.0, 1.0); }
            7 => { params.lfo_rate = (params.lfo_rate + dir as f32 * 0.5).clamp(0.0, 20.0); }
            8 => { params.lfo_depth = (params.lfo_depth + dir as f32 * 0.02).clamp(0.0, 1.0); }
            9 => { params.duration_ms = (params.duration_ms as i64 + dir as i64 * 20).clamp(10, 5000) as u64; }
            _ => {}
        }
    }

    // ── Canvas ─────────────────────────────────────────────────────

    fn handle_canvas_key(&mut self, key: KeyEvent) {
        match self.canvas.mode {
            CanvasMode::PiecePicker => self.handle_canvas_piece_picker(key),
            CanvasMode::Drawing => self.handle_canvas_drawing(key),
            CanvasMode::ShapePicker => self.handle_canvas_shape_picker(key),
        }
    }

    fn handle_canvas_piece_picker(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.canvas.selected_piece > 0 { self.canvas.selected_piece -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.canvas.selected_piece < PIECE_TYPES.len() - 1 { self.canvas.selected_piece += 1; }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                self.canvas.load_grid(&self.custom_pieces);
                self.canvas.mode = CanvasMode::Drawing;
                self.canvas.cursor_row = 1;
                self.canvas.cursor_col = 3;
                let name = self.canvas.piece_name();
                let ch = self.canvas.current_char();
                self.message = format!("Drawing {name} | Shape: {ch} | Enter=stamp Space=erase Tab=shapes [s]ave Esc=back");
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Menu;
                self.message = String::from("hjkl/arrows to navigate, Enter to select");
            }
            _ => {}
        }
    }

    fn handle_canvas_drawing(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => { if self.canvas.cursor_row > 0 { self.canvas.cursor_row -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.canvas.cursor_row < 2 { self.canvas.cursor_row += 1; } }
            KeyCode::Left | KeyCode::Char('h') => { if self.canvas.cursor_col > 0 { self.canvas.cursor_col -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if self.canvas.cursor_col < 6 { self.canvas.cursor_col += 1; } }
            KeyCode::Enter => { self.canvas.stamp(); }
            KeyCode::Char(' ') | KeyCode::Delete | KeyCode::Backspace => { self.canvas.erase(); }
            KeyCode::Tab => {
                self.canvas.mode = CanvasMode::ShapePicker;
                self.message = String::from("Select a shape | hjkl/arrows | Enter to pick | Esc=back");
            }
            KeyCode::Char('s') => {
                let pt = self.canvas.piece_type();
                self.custom_pieces.set(pt, self.canvas.grid);
                match self.custom_pieces.save() {
                    Ok(_) => { self.message = format!("{} saved!", self.canvas.piece_name()); }
                    Err(e) => { self.message = format!("Save failed: {e}"); }
                }
            }
            KeyCode::Char('c') => {
                self.canvas.grid = [[' '; 7]; 3];
                self.message = String::from("Grid cleared");
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.canvas.mode = CanvasMode::PiecePicker;
                self.message = String::from("Select a piece to draw");
            }
            _ => {}
        }
    }

    fn handle_canvas_shape_picker(&mut self, key: KeyEvent) {
        let last = SHAPE_PALETTE.len() - 1;
        let step = 20;
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => { if self.canvas.current_shape > 0 { self.canvas.current_shape -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if self.canvas.current_shape < last { self.canvas.current_shape += 1; } }
            KeyCode::Up | KeyCode::Char('k') => { self.canvas.current_shape = self.canvas.current_shape.saturating_sub(step); }
            KeyCode::Down | KeyCode::Char('j') => { self.canvas.current_shape = (self.canvas.current_shape + step).min(last); }
            KeyCode::Enter | KeyCode::Esc => {
                self.canvas.mode = CanvasMode::Drawing;
                let name = self.canvas.piece_name();
                let ch = self.canvas.current_char();
                self.message = format!("Drawing {name} | Shape: {ch} | Enter=stamp Space=erase Tab=shapes [s]ave Esc=back");
            }
            _ => {}
        }
    }
}
