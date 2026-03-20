use std::path::Path;
use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::audio::Audio;
use crate::board::{Color, Move, Position, QUEEN};
use crate::canvas::{CanvasMode, CanvasState, CustomPieces, PIECE_TYPES, SHAPE_PALETTE};
use crate::net::NetClient;
use crate::protocol::*;
use crate::engine::{self, Personality, PERSONALITIES};
use crate::lessons::STUDY_CATEGORIES;
use crate::minigames::{KnightTour, ColorQuiz, MINIGAME_LIST};
use crate::puzzle::{Puzzle, PuzzleIndex, TACTIC_THEMES};
use crate::settings::{Settings, SETTINGS_ITEMS, SOUND_EVENT_NAMES, SYNTH_PARAM_NAMES};
use crate::identity;
use crate::tracker::{self, RemoteServer};

#[allow(dead_code)]
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
    RoomNameInput,
    TimeControlSelect,
    StudyMenu,
    LessonList,
    LessonView,
    MiniGameMenu,
    KnightTourGame,
    ColorQuizGame,
    ComputerSelect,
    ComputerGame,
}

pub struct ChatState {
    pub messages: Vec<(String, String, ChatKind)>,
    pub input: String,
    pub typing: bool,
    pub scroll: usize, // 0 = bottom (newest), >0 = scrolled up
}

impl ChatState {
    fn new() -> Self {
        Self { messages: Vec::new(), input: String::new(), typing: false, scroll: 0 }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Board,
    Panel,
}

pub struct App {
    pub screen: Screen,
    pub focus: Focus,
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
    pub last_move: Option<(u8, u8)>,
    pub move_history: Vec<String>,
    pub flipped: bool,
    pub white_time_ms: u64,
    pub black_time_ms: u64,
    pub time_control: TimeControl,
    pub time_control_selection: usize,
    pub clock_tick: std::time::Instant,  // last time we ticked the display clock
    // Discovery (reserved for future LAN mode)
    #[allow(dead_code)]
    pub remote_servers: Vec<RemoteServer>,
    #[allow(dead_code)]
    pub heartbeat_tx: Option<mpsc::Sender<u32>>,
    #[allow(dead_code)]
    pub public_ip: Option<String>,
    // Audio & Settings
    pub audio: Option<Audio>,
    pub settings: Settings,
    pub settings_selection: usize,
    pub sound_event_selection: usize,
    pub sound_param_selection: usize,
    pub name_input: String,
    pub room_name_input: String,
    pub client_id: String,
    // Study
    pub study_category: usize,
    pub study_lesson: usize,
    pub study_move: usize,
    pub study_positions: Vec<Position>,
    // Mini-games
    pub minigame_selection: usize,
    pub knight_tour: Option<KnightTour>,
    pub color_quiz: Option<ColorQuiz>,
    // Computer game
    pub computer_selection: usize,
    pub computer_personality: Option<Personality>,
    pub player_color: Color,
    pub computer_thinking: bool,
}

const MENU_ITEMS: &[&str] = &[
    "Play Computer",
    "Practice Tactics",
    "Study",
    "Mini-Games",
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
            focus: Focus::Panel, // start with panel focused so menu is navigable
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
            last_move: None,
            move_history: Vec::new(),
            flipped: false,
            white_time_ms: 0,
            black_time_ms: 0,
            time_control: TimeControl::None,
            time_control_selection: 0,
            clock_tick: std::time::Instant::now(),
            remote_servers: Vec::new(),
            heartbeat_tx: None,
            public_ip: None,
            audio: Audio::new(),
            settings,
            settings_selection: 0,
            sound_event_selection: 0,
            sound_param_selection: 0,
            name_input: String::new(),
            room_name_input: String::new(),
            client_id: identity::get_or_create_client_id(data_dir),
            study_category: 0,
            study_lesson: 0,
            study_move: 0,
            study_positions: Vec::new(),
            minigame_selection: 0,
            knight_tour: None,
            color_quiz: None,
            computer_selection: 0,
            computer_personality: None,
            player_color: Color::White,
            computer_thinking: false,
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

    /// Update the status message to show controls for current state.
    fn update_hint(&mut self) {
        let focus_label = match self.focus {
            Focus::Board => "[Board]",
            Focus::Panel => "[Panel]",
        };
        self.message = match self.screen {
            Screen::Menu | Screen::Analysis => {
                match self.focus {
                    Focus::Board => format!("{focus_label} hjkl=move Enter=select piece Tab=panel Esc=deselect q=quit"),
                    Focus::Panel => format!("{focus_label} jk=navigate Enter=select Tab=board q=quit"),
                }
            }
            Screen::ThemePicker => format!("{focus_label} jk=navigate Enter=select Esc=back"),
            Screen::Puzzle => {
                match self.focus {
                    Focus::Board => format!("{focus_label} hjkl=move Enter=select/move H=hint n=next Esc=back"),
                    Focus::Panel => format!("{focus_label} Tab=board H=hint n=next Esc=back"),
                }
            }
            Screen::Results => format!("Enter=menu"),
            Screen::RoomBrowser => format!("{focus_label} jk=navigate Enter=join n=new room r=refresh Esc=back"),
            Screen::RoomLobby => {
                if self.chat.typing {
                    String::from("Type message, Enter=send, Esc=stop")
                } else {
                    format!("{focus_label} jk=navigate Enter=join t=new table Tab=chat Esc=leave")
                }
            }
            Screen::LiveGame => {
                if self.chat.typing {
                    String::from("Type message, Enter=send, Esc=stop")
                } else {
                    match self.focus {
                        Focus::Board => format!("{focus_label} hjkl=move Enter=select/move Tab=chat r=resign Esc=leave"),
                        Focus::Panel => format!("{focus_label} Tab=board/chat r=resign Esc=leave"),
                    }
                }
            }
            Screen::Settings => format!("{focus_label} jk=navigate Enter=select Esc=back"),
            Screen::SoundSettings => format!("{focus_label} jk=navigate Enter=edit m=mute Esc=back"),
            Screen::SoundEventEdit => format!("{focus_label} jk=param hl=adjust p=preview s=save Esc=back"),
            Screen::NameEdit => format!("Type name, Enter=save, Esc=cancel"),
            Screen::RoomNameInput => format!("Type room name, Enter=create, Esc=cancel"),
            Screen::TimeControlSelect => format!("jk=navigate Enter=select Esc=cancel"),
            Screen::StudyMenu => format!("{focus_label} jk=navigate Enter=select Esc=back"),
            Screen::LessonList => format!("{focus_label} jk=navigate Enter=start Esc=back"),
            Screen::LessonView => format!("{focus_label} n/Space=next b/Bksp=back Esc=list"),
            Screen::ComputerSelect => format!("{focus_label} jk=navigate Enter=play Esc=back"),
            Screen::ComputerGame => {
                match self.focus {
                    Focus::Board => format!("{focus_label} hjkl=move Enter=select/move Esc=back"),
                    Focus::Panel => format!("{focus_label} Tab=board Esc=back"),
                }
            }
            Screen::MiniGameMenu => format!("{focus_label} jk=navigate Enter=play Esc=back"),
            Screen::KnightTourGame => format!("{focus_label} hjkl=move Enter=place knight b=undo Esc=quit"),
            Screen::ColorQuizGame => format!("l=light d=dark Esc=quit"),
            Screen::Canvas => String::new(), // canvas has its own hints
        };
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

    /// Tick the display clock down for the active player. Call every loop iteration.
    pub fn tick_clock(&mut self) {
        if !self.game_active || self.time_control == TimeControl::None {
            return;
        }
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.clock_tick).as_millis() as u64;
        self.clock_tick = now;

        match self.board.side_to_move {
            crate::board::Color::White => {
                self.white_time_ms = self.white_time_ms.saturating_sub(elapsed);
            }
            crate::board::Color::Black => {
                self.black_time_ms = self.black_time_ms.saturating_sub(elapsed);
            }
        }
    }

    /// Check if network connection is alive.
    fn is_connected(&self) -> bool {
        self.net.is_some() && self.my_id.is_some()
    }

    /// Try to reconnect to the central server.
    fn try_reconnect(&mut self) {
        self.net = None;
        self.net_rx = None;
        self.my_id = None;
        match NetClient::connect() {
            Ok((client, rx)) => {
                self.net = Some(client);
                self.net_rx = Some(rx);
                self.message = String::from("Reconnecting...");
            }
            Err(_) => {
                self.message = String::from("Could not connect. Try again.");
            }
        }
    }

    /// Called each tick from the event loop to drain network messages.
    pub fn poll_network(&mut self) {
        let msgs: Vec<ServerMsg> = if let Some(ref rx) = self.net_rx {
            let mut v = Vec::new();
            loop {
                match rx.try_recv() {
                    Ok(msg) => v.push(msg),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // Connection lost
                        self.net = None;
                        self.net_rx = None;
                        self.my_id = None;
                        self.message = String::from("Connection lost.");
                        break;
                    }
                }
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
                self.focus = Focus::Board;
                self.game_active = false;
                let has_opponent = table.white.is_some() && table.black.is_some();
                if has_opponent {
                    self.message = String::from("Game starting...");
                } else {
                    self.message = String::from("Waiting for opponent to join...");
                }
            }
            ServerMsg::GameStarted { table_id, white, black, fen, time_control } => {
                self.live_white = Some(white);
                self.live_black = Some(black);
                self.game_active = true;
                self.last_move = None;
                self.move_history.clear();
                self.time_control = time_control;
                let initial = match time_control {
                    TimeControl::None => 0,
                    TimeControl::Minutes(m) => m as u64 * 60 * 1000,
                };
                self.white_time_ms = initial;
                self.black_time_ms = initial;
                self.clock_tick = std::time::Instant::now();
                if let Some(pos) = Position::from_fen(&fen) {
                    self.board = pos;
                }
                self.selected_sq = None;
                self.highlights.clear();

                // Flip board if playing black
                let is_black = Some(black) == self.my_id;
                self.flipped = is_black;
                // Start cursor in center from player's perspective
                self.cursor = if is_black { 35 } else { 28 };

                if self.current_table != Some(table_id) {
                    return;
                }
                self.screen = Screen::LiveGame;
                self.focus = Focus::Board;
                self.play_sound(|a, s| a.play_login(s));

                let my_color = if Some(white) == self.my_id {
                    "White — your move!"
                } else if is_black {
                    "Black — waiting for White"
                } else {
                    "Spectating"
                };
                self.message = format!("Game started! {my_color}");
            }
            ServerMsg::MoveMade { table_id, uci, fen, white_time_ms, black_time_ms } => {
                if self.current_table == Some(table_id) {
                    // Track last move for highlighting
                    if let Some(mv) = Move::from_uci(&uci) {
                        self.last_move = Some((mv.from, mv.to));
                    }
                    self.move_history.push(uci);
                    self.white_time_ms = white_time_ms;
                    self.black_time_ms = black_time_ms;
                    self.clock_tick = std::time::Instant::now();

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
                    self.focus = Focus::Board;

                    if self.is_my_turn() {
                        let check = if self.board.in_check(self.board.side_to_move) { " You're in check!" } else { "" };
                        self.message = format!("Your move!{check}");
                    } else {
                        self.message = String::from("Waiting for opponent...");
                    }
                }
            }
            ServerMsg::GameOver { table_id, reason, winner } => {
                if self.current_table == Some(table_id) {
                    let is_player = self.live_white == self.my_id || self.live_black == self.my_id;
                    let result = match winner {
                        Some(id) if Some(id) == self.my_id => "You win!",
                        Some(_) if is_player => "You lose.",
                        Some(_) => "Game over.",
                        None => "Draw.",
                    };
                    self.play_sound(|a, s| a.play_checkmate(s));
                    self.game_active = false;
                    if is_player {
                        self.message = format!("{reason} — {result} r=rematch Esc=leave");
                    } else {
                        self.message = format!("{reason} — {result} Esc=leave");
                    }
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
        // Global: Ctrl+C / Ctrl+Q always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => {
                    self.running = false;
                    return;
                }
                _ => {}
            }
        }

        // Canvas has its own controls (full screen, no focus model)
        if let Screen::Canvas = self.screen {
            self.handle_canvas_key(key);
            return;
        }

        // Text input screens — consume all keys
        match self.screen {
            Screen::NameEdit => { self.handle_name_edit_key(key); return; }
            Screen::RoomNameInput => { self.handle_room_name_input_key(key); return; }
            Screen::TimeControlSelect => { self.handle_time_control_select_key(key); return; }
            _ => {}
        }

        // Chat typing mode — consume keys for text
        if self.chat.typing {
            match key.code {
                KeyCode::Enter => {
                    if !self.chat.input.is_empty() {
                        let body = self.chat.input.clone();
                        self.chat.input.clear();
                        self.chat.scroll = 0; // snap to bottom on send
                        self.send_net(ClientMsg::SendChat { body });
                    }
                }
                KeyCode::Char(c) => { self.chat.input.push(c); }
                KeyCode::Backspace => { self.chat.input.pop(); }
                KeyCode::Up => {
                    // Scroll chat up
                    if self.chat.scroll < self.chat.messages.len() {
                        self.chat.scroll += 1;
                    }
                }
                KeyCode::Down => {
                    // Scroll chat down
                    if self.chat.scroll > 0 {
                        self.chat.scroll -= 1;
                    }
                }
                KeyCode::Esc => {
                    self.chat.typing = false;
                    self.chat.scroll = 0;
                    self.update_hint();
                }
                _ => {}
            }
            return;
        }

        // Global keys: Tab toggles focus, q quits from menu
        match key.code {
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Board => Focus::Panel,
                    Focus::Panel => Focus::Board,
                };
                // In lobby/live game, Tab from Panel enters chat
                match self.screen {
                    Screen::RoomLobby | Screen::LiveGame => {
                        if self.focus == Focus::Panel {
                            self.chat.typing = true;
                            self.message = String::from("Type message, Enter=send, Esc=stop");
                            return;
                        }
                    }
                    _ => {}
                }
                self.update_hint();
                return;
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                if let Screen::Menu = self.screen {
                    self.running = false;
                    return;
                }
            }
            _ => {}
        }

        // Route based on focus
        match self.focus {
            Focus::Board => self.handle_board_input(key),
            Focus::Panel => self.handle_panel_input(key),
        }
    }

    /// Board-focused input: cursor movement, piece selection
    fn handle_board_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            | KeyCode::Char('h') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('l') => {
                self.move_cursor(key);
            }
            KeyCode::Enter => {
                match self.screen {
                    Screen::Menu | Screen::Analysis => {
                        // Free piece movement
                        if self.selected_sq.is_some() {
                            self.handle_free_move();
                        } else if self.board.piece_at(self.cursor).is_some() {
                            self.select_piece(self.cursor);
                        }
                    }
                    Screen::Puzzle => { self.handle_puzzle_select(); }
                    Screen::LiveGame => {
                        if self.is_my_turn() { self.handle_live_game_select(); }
                    }
                    Screen::ComputerGame => {
                        if self.board.side_to_move == self.player_color && !self.computer_thinking {
                            self.handle_computer_move_select();
                        }
                    }
                    Screen::KnightTourGame => {
                        let result = if let Some(ref mut tour) = self.knight_tour {
                            let sq = self.cursor;
                            if tour.try_move(sq) {
                                self.board = tour.to_position();
                                self.highlights = tour.visited_mask();
                                let complete = tour.is_complete();
                                let stuck = tour.is_stuck();
                                let count = tour.visit_count;
                                let moves = tour.move_history.len();
                                Some((true, complete, stuck, count, moves))
                            } else {
                                Some((false, false, false, 0, 0))
                            }
                        } else { None };

                        if let Some((moved, complete, stuck, count, moves)) = result {
                            if moved {
                                self.play_sound(|a, s| a.play_move(s));
                                if complete {
                                    self.play_sound(|a, s| a.play_session_complete(s));
                                    self.message = format!("COMPLETE! All 64 squares in {moves} moves!");
                                } else if stuck {
                                    self.message = format!("Stuck at {count}/64! Press b to undo.");
                                } else {
                                    self.message = format!("Squares: {count}/64 — {} remaining", 64 - count);
                                }
                            } else {
                                self.play_sound(|a, s| a.play_wrong(s));
                                self.message = String::from("Can't move there — knight moves in L shape");
                            }
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Esc => {
                self.selected_sq = None;
                self.highlights.clear();
                self.update_hint();
            }
            // Screen-specific shortcuts that work from board focus
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char(' ') => {
                if let Screen::LessonView = self.screen {
                    self.study_step_forward();
                    return;
                }
                if let Screen::Puzzle = self.screen {
                    self.score_total += 1;
                    self.advance_puzzle();
                }
            }
            KeyCode::Char('H') => {
                if let Screen::Puzzle = self.screen {
                    if let Some(puzzle) = self.puzzle_queue.get(self.puzzle_pos) {
                        if self.puzzle_move_index < puzzle.moves.len() {
                            let hint_move = puzzle.moves[self.puzzle_move_index].clone();
                            self.message = format!("Hint: {hint_move}");
                            if let Some(mv) = Move::from_uci(&hint_move) {
                                self.highlights = vec![mv.from, mv.to];
                            }
                            self.play_sound(|a, s| a.play_hint(s));
                        }
                    }
                }
            }
            KeyCode::Char('b') | KeyCode::Backspace => {
                if let Screen::LessonView = self.screen {
                    self.study_step_back();
                }
                if let Screen::KnightTourGame = self.screen {
                    if let Some(ref mut tour) = self.knight_tour {
                        tour.undo();
                        self.board = tour.to_position();
                        self.highlights = tour.visited_mask();
                        self.message = format!("Squares: {}/64 — Undo!", tour.visit_count);
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Screen::LiveGame = self.screen {
                    if self.game_active {
                        self.send_net(ClientMsg::Resign);
                    } else {
                        // Game over — request rematch (swaps colors)
                        self.send_net(ClientMsg::Rematch);
                        self.message = String::from("Rematch requested...");
                    }
                }
            }
            _ => {}
        }
    }

    /// Panel-focused input: menu/list navigation, Enter activates
    fn handle_panel_input(&mut self, key: KeyEvent) {
        match self.screen {
            Screen::Menu | Screen::Analysis => self.handle_menu_panel(key),
            Screen::ThemePicker => self.handle_theme_picker_key(key),
            Screen::Puzzle => self.handle_puzzle_panel(key),
            Screen::Results => self.handle_results_key(key),
            Screen::RoomBrowser => self.handle_room_browser_key(key),
            Screen::RoomLobby => self.handle_room_lobby_panel(key),
            Screen::LiveGame => {
                // Panel in live game is chat — Tab already handles entering chat
                // Esc goes back
                if key.code == KeyCode::Esc {
                    self.send_net(ClientMsg::LeaveTable);
                    self.current_table = None;
                    self.game_active = false;
                    self.screen = Screen::RoomLobby;
                    self.focus = Focus::Panel;
                    self.update_hint();
                }
            }
            Screen::Settings => self.handle_settings_key(key),
            Screen::SoundSettings => self.handle_sound_settings_key(key),
            Screen::SoundEventEdit => self.handle_sound_event_edit_key(key),
            Screen::StudyMenu => self.handle_study_menu_key(key),
            Screen::LessonList => self.handle_lesson_list_key(key),
            Screen::LessonView => self.handle_lesson_view_panel_key(key),
            Screen::ComputerSelect => self.handle_computer_select_key(key),
            Screen::ComputerGame => self.handle_computer_game_panel_key(key),
            Screen::MiniGameMenu => self.handle_minigame_menu_key(key),
            Screen::ColorQuizGame => self.handle_color_quiz_key(key),
            Screen::KnightTourGame => {
                // Panel Esc to quit
                if key.code == KeyCode::Esc {
                    self.knight_tour = None;
                    self.screen = Screen::MiniGameMenu;
                    self.message = String::from("Choose a mini-game");
                }
            }
            _ => {}
        }
    }

    fn handle_menu_panel(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.menu_selection > 0 { self.menu_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.menu_selection < MENU_ITEMS.len() - 1 { self.menu_selection += 1; }
            }
            KeyCode::Enter => {
                self.menu_select(self.menu_selection);
            }
            _ => {}
        }
    }

    fn handle_puzzle_panel(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.score_total += 1;
                self.advance_puzzle();
            }
            KeyCode::Char('H') => {
                if let Some(puzzle) = self.puzzle_queue.get(self.puzzle_pos) {
                    if self.puzzle_move_index < puzzle.moves.len() {
                        let hint_move = puzzle.moves[self.puzzle_move_index].clone();
                        self.message = format!("Hint: {hint_move}");
                        if let Some(mv) = Move::from_uci(&hint_move) {
                            self.highlights = vec![mv.from, mv.to];
                        }
                        self.play_sound(|a, s| a.play_hint(s));
                    }
                }
            }
            KeyCode::Esc => {
                self.selected_sq = None;
                self.highlights.clear();
                self.screen = Screen::ThemePicker;
                self.focus = Focus::Panel;
                self.update_hint();
            }
            _ => {}
        }
    }

    fn handle_room_lobby_panel(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.table_selection > 0 { self.table_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.tables.is_empty() && self.table_selection < self.tables.len() - 1 {
                    self.table_selection += 1;
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.time_control_selection = 0;
                self.screen = Screen::TimeControlSelect;
                self.message = String::from("Select time control for your table");
            }
            KeyCode::Enter => {
                if let Some(table) = self.tables.get(self.table_selection) {
                    self.send_net(ClientMsg::JoinTable { table_id: table.id });
                }
            }
            KeyCode::Esc => {
                self.send_net(ClientMsg::LeaveRoom);
                self.current_room = None;
                self.room_players.clear();
                self.tables.clear();
                self.send_net(ClientMsg::ListRooms);
                self.screen = Screen::RoomBrowser;
                self.focus = Focus::Panel;
                self.update_hint();
            }
            _ => {}
        }
    }

    fn handle_free_move(&mut self) {
        let to = self.cursor;
        if let Some(from) = self.selected_sq {
            if from == to {
                self.selected_sq = None;
                self.highlights.clear();
                return;
            }
            // Move piece freely (no rule validation)
            if let Some((pt, color)) = self.board.piece_at(from) {
                let from_bit = 1u64 << from;
                let to_bit = 1u64 << to;
                let ci = if color == crate::board::Color::White { crate::board::WHITE } else { crate::board::BLACK };
                // Remove from origin
                self.board.pieces[pt] &= !from_bit;
                self.board.colors[ci] &= !from_bit;
                // Remove any piece at destination
                for p in 0..6 {
                    self.board.pieces[p] &= !to_bit;
                }
                self.board.colors[0] &= !to_bit;
                self.board.colors[1] &= !to_bit;
                // Place at destination
                self.board.pieces[pt] |= to_bit;
                self.board.colors[ci] |= to_bit;
                self.play_sound(|a, s| a.play_move(s));
            }
            self.selected_sq = None;
            self.highlights.clear();
        }
    }

    fn menu_select(&mut self, idx: usize) {
        match idx {
            0 => {
                    self.computer_selection = 0;
                    self.screen = Screen::ComputerSelect;
                    self.focus = Focus::Panel;
                    self.message = String::from("Choose your opponent");
                }
                1 => {
                    self.screen = Screen::ThemePicker;
                    self.theme_selection = 0;
                    self.focus = Focus::Panel;
                    self.message = String::from("Pick a tactic theme");
                }
                2 => {
                    self.study_category = 0;
                    self.screen = Screen::StudyMenu;
                    self.focus = Focus::Panel;
                    self.message = String::from("Choose what to study");
                }
                3 => {
                    self.minigame_selection = 0;
                    self.screen = Screen::MiniGameMenu;
                    self.focus = Focus::Panel;
                    self.message = String::from("Choose a mini-game");
                }
                4 => {
                    self.board = Position::start();
                    self.selected_sq = None;
                    self.highlights.clear();
                    self.message = String::from("Move pieces freely. Tab=menu, Enter=select/place.");
                }
                5 => {
                    // Go Online — connect to central game server
                    if self.net.is_some() {
                        self.send_net(ClientMsg::ListRooms);
                        self.screen = Screen::RoomBrowser;
                        self.focus = Focus::Panel;
                        self.update_hint();
                    } else {
                        self.message = String::from("Connecting...");
                        match NetClient::connect() {
                            Ok((client, rx)) => {
                                self.net = Some(client);
                                self.net_rx = Some(rx);
                                // Welcome message will trigger screen transition
                            }
                            Err(e) => {
                                self.message = format!("Connection failed: {e}");
                            }
                        }
                    }
                }
                6 => {
                    self.settings_selection = 0;
                    self.screen = Screen::Settings;
                    self.message = String::from("Settings");
                }
                7 => { self.running = false; }
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

                // Try local index first, fall back to server
                let puzzles = if let Some(ref index) = self.puzzle_index {
                    index.load_theme(theme_tag, Some(2000), PUZZLE_BATCH_SIZE).unwrap_or_default()
                } else {
                    self.message = String::from("Fetching puzzles from server...");
                    tracker::fetch_puzzles(&self.client_id, theme_tag, 2000, PUZZLE_BATCH_SIZE, 0)
                };

                if puzzles.is_empty() {
                    self.message = format!("No puzzles found for '{theme_name}'");
                } else {
                    self.puzzle_queue = puzzles;
                    self.puzzle_pos = 0;
                    self.score_correct = 0;
                    self.score_total = 0;
                    self.load_current_puzzle();
                    self.screen = Screen::Puzzle;
                    self.focus = Focus::Board;
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
        // When board is flipped, visual directions are reversed
        let (up, down, left, right) = if self.flipped {
            (-8i8, 8i8, 1i8, -1i8)
        } else {
            (8, -8, -1, 1)
        };
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                let new_file = file as i8 + left;
                if new_file >= 0 && new_file < 8 { self.cursor = (self.cursor as i8 + left) as u8; }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let new_file = file as i8 + right;
                if new_file >= 0 && new_file < 8 { self.cursor = (self.cursor as i8 + right) as u8; }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let new_rank = rank as i8 + (up / up.abs());
                if new_rank >= 0 && new_rank < 8 { self.cursor = (self.cursor as i8 + up) as u8; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let new_rank = rank as i8 + (down / down.abs());
                if new_rank >= 0 && new_rank < 8 { self.cursor = (self.cursor as i8 + down) as u8; }
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
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.room_selection > 0 { self.room_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.room_list.is_empty() && self.room_selection < self.room_list.len() - 1 {
                    self.room_selection += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(room) = self.room_list.get(self.room_selection) {
                    self.send_net(ClientMsg::JoinRoom { room_id: room.id });
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if !self.is_connected() {
                    self.try_reconnect();
                    return;
                }
                self.room_name_input = format!("{}'s room", self.player_name);
                self.screen = Screen::RoomNameInput;
                self.message = String::from("Name your room, Enter to create, Esc to cancel");
            }
            KeyCode::Char('r') => {
                if !self.is_connected() {
                    self.try_reconnect();
                }
                if !self.is_connected() {
                    self.try_reconnect();
                } else {
                    self.send_net(ClientMsg::ListRooms);
                    self.message = String::from("Refreshed.");
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Menu;
                self.message = String::from("hjkl/arrows to navigate, Enter to select");
            }
            _ => {}
        }
    }

    fn handle_room_name_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.room_name_input.is_empty() {
                    let name = self.room_name_input.clone();
                    self.send_net(ClientMsg::CreateRoom { name });
                    // Server will respond with RoomJoined
                }
            }
            KeyCode::Char(c) => { self.room_name_input.push(c); }
            KeyCode::Backspace => { self.room_name_input.pop(); }
            KeyCode::Esc => {
                self.screen = Screen::RoomBrowser;
                self.message = String::from("[n]ew room, [r]efresh, Enter=join");
            }
            _ => {}
        }
    }

    fn handle_time_control_select_key(&mut self, key: KeyEvent) {
        const OPTIONS: &[(&str, TimeControl)] = &[
            ("No Clock (unlimited)", TimeControl::None),
            ("5 Minutes", TimeControl::Minutes(5)),
            ("10 Minutes", TimeControl::Minutes(10)),
            ("20 Minutes", TimeControl::Minutes(20)),
            ("30 Minutes", TimeControl::Minutes(30)),
        ];
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.time_control_selection > 0 { self.time_control_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.time_control_selection < OPTIONS.len() - 1 { self.time_control_selection += 1; }
            }
            KeyCode::Enter => {
                let (name, tc) = OPTIONS[self.time_control_selection];
                self.send_net(ClientMsg::CreateTable { time_control: tc });
                self.screen = Screen::RoomLobby;
                self.message = format!("Creating {name} table...");
            }
            KeyCode::Esc => {
                self.screen = Screen::RoomLobby;
                self.update_hint();
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

    // ── Computer Game ──────────────────────────────────────────────

    fn handle_computer_select_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.computer_selection > 0 { self.computer_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.computer_selection < PERSONALITIES.len() - 1 { self.computer_selection += 1; }
            }
            KeyCode::Enter => {
                let personality = PERSONALITIES[self.computer_selection].clone();
                self.board = Position::start();
                self.selected_sq = None;
                self.highlights.clear();
                self.cursor = 28;
                self.player_color = Color::White;
                self.computer_thinking = false;
                let name = personality.name;
                self.computer_personality = Some(personality);
                self.screen = Screen::ComputerGame;
                self.focus = Focus::Board;
                self.message = format!("Playing as White vs {name}. Your move!");
            }
            KeyCode::Esc => {
                self.screen = Screen::Menu;
                self.update_hint();
            }
            _ => {}
        }
    }

    fn handle_computer_game_panel_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.computer_personality = None;
            self.screen = Screen::Menu;
            self.board = Position::start();
            self.update_hint();
        }
    }

    fn handle_computer_move_select(&mut self) {
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
                    && (m.promotion.is_none() || m.promotion == Some(QUEEN))
            });
            if let Some(&mv) = mv {
                // Play the move
                let is_capture = self.board.piece_at(mv.to).is_some();
                self.board = self.board.make_move(mv);
                self.selected_sq = None;
                self.highlights.clear();

                if is_capture {
                    self.play_sound(|a, s| a.play_capture(s));
                } else {
                    self.play_sound(|a, s| a.play_move(s));
                }

                // Check game over
                if self.board.is_checkmate() {
                    self.play_sound(|a, s| a.play_checkmate(s));
                    self.message = String::from("Checkmate! You win!");
                    self.computer_personality = None;
                    return;
                }
                if self.board.is_stalemate() {
                    self.message = String::from("Stalemate! Draw.");
                    self.computer_personality = None;
                    return;
                }
                if self.board.in_check(self.board.side_to_move) {
                    self.play_sound(|a, s| a.play_check(s));
                }

                // Computer's turn
                self.computer_thinking = true;
                self.message = String::from("Computer is thinking...");
            } else {
                // Maybe selecting a different piece
                if let Some((_, color)) = self.board.piece_at(sq) {
                    if color == self.player_color {
                        self.select_piece(sq);
                        return;
                    }
                }
            }
        } else {
            if let Some((_, color)) = self.board.piece_at(sq) {
                if color == self.player_color {
                    self.select_piece(sq);
                }
            }
        }
    }

    /// Called from the event loop to let the computer make its move.
    pub fn computer_think(&mut self) {
        if !self.computer_thinking { return; }
        if self.computer_personality.is_none() { return; }

        let personality = self.computer_personality.as_ref().unwrap();
        if let Some(mv) = engine::find_best_move(&self.board, personality) {
            let is_capture = self.board.piece_at(mv.to).is_some();
            self.board = self.board.make_move(mv);

            if is_capture {
                self.play_sound(|a, s| a.play_capture(s));
            } else {
                self.play_sound(|a, s| a.play_move(s));
            }

            self.computer_thinking = false;

            if self.board.is_checkmate() {
                self.play_sound(|a, s| a.play_checkmate(s));
                let name = personality.name;
                self.message = format!("Checkmate! {name} wins.");
                self.computer_personality = None;
                return;
            }
            if self.board.is_stalemate() {
                self.message = String::from("Stalemate! Draw.");
                self.computer_personality = None;
                return;
            }
            if self.board.in_check(self.board.side_to_move) {
                self.play_sound(|a, s| a.play_check(s));
                self.message = String::from("Check! Your move.");
            } else {
                self.message = String::from("Your move!");
            }
        }
    }

    // ── Mini-Games ─────────────────────────────────────────────────

    fn handle_minigame_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.minigame_selection > 0 { self.minigame_selection -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.minigame_selection < MINIGAME_LIST.len() - 1 { self.minigame_selection += 1; }
            }
            KeyCode::Enter => {
                match self.minigame_selection {
                    0 => {
                        // Knight's Tour
                        let tour = KnightTour::new(0); // start at a1
                        self.board = tour.to_position();
                        self.highlights = tour.visited_mask();
                        self.cursor = 0;
                        self.knight_tour = Some(tour);
                        self.screen = Screen::KnightTourGame;
                        self.focus = Focus::Board;
                        self.message = String::from("Knight's Tour! Visit all 64 squares. Enter=move b=undo");
                    }
                    1 => {
                        // Color Quiz
                        self.color_quiz = Some(ColorQuiz::new());
                        self.screen = Screen::ColorQuizGame;
                        self.focus = Focus::Panel;
                        let sq_name = self.color_quiz.as_ref().unwrap().square_name();
                        self.message = format!("Is {sq_name} LIGHT or DARK? Press l=light d=dark");
                    }
                    2 => {
                        // Blindfold — just toggle piece visibility on the main board
                        self.message = String::from("Blindfold mode coming soon!");
                    }
                    _ => {}
                }
            }
            KeyCode::Esc => {
                self.screen = Screen::Menu;
                self.update_hint();
            }
            _ => {}
        }
    }

    fn handle_color_quiz_key(&mut self, key: KeyEvent) {
        let guess = match key.code {
            KeyCode::Char('l') | KeyCode::Char('L') => Some(true),
            KeyCode::Char('d') | KeyCode::Char('D') => Some(false),
            KeyCode::Esc => {
                self.color_quiz = None;
                self.screen = Screen::MiniGameMenu;
                self.message = String::from("Choose a mini-game");
                return;
            }
            _ => None,
        };

        if let Some(light) = guess {
            let result = if let Some(ref mut quiz) = self.color_quiz {
                let was_light = quiz.is_light();
                let correct = quiz.guess(light);
                let sq_name = quiz.square_name();
                Some((correct, was_light, quiz.score, quiz.total, quiz.streak, sq_name))
            } else { None };

            if let Some((correct, was_light, score, total, streak, sq_name)) = result {
                if correct {
                    self.play_sound(|a, s| a.play_correct(s));
                    self.message = format!("Correct! {score}/{total} Streak: {streak} — Is {sq_name} light or dark?");
                } else {
                    self.play_sound(|a, s| a.play_wrong(s));
                    let was = if was_light { "light" } else { "dark" };
                    self.message = format!("Wrong! It was {was}. {score}/{total} — Is {sq_name} light or dark?");
                }
            }
        }
    }

    // ── Study ──────────────────────────────────────────────────────

    fn handle_study_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.study_category > 0 { self.study_category -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.study_category < STUDY_CATEGORIES.len() - 1 { self.study_category += 1; }
            }
            KeyCode::Enter => {
                self.study_lesson = 0;
                self.screen = Screen::LessonList;
                self.message = String::from("Select a lesson");
            }
            KeyCode::Esc => {
                self.screen = Screen::Menu;
                self.update_hint();
            }
            _ => {}
        }
    }

    fn handle_lesson_list_key(&mut self, key: KeyEvent) {
        let category = &STUDY_CATEGORIES[self.study_category];
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.study_lesson > 0 { self.study_lesson -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.study_lesson < category.lessons.len() - 1 { self.study_lesson += 1; }
            }
            KeyCode::Enter => {
                self.start_lesson();
            }
            KeyCode::Esc => {
                self.screen = Screen::StudyMenu;
                self.message = String::from("Choose what to study");
            }
            _ => {}
        }
    }

    fn handle_lesson_view_panel_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char(' ') => {
                self.study_step_forward();
            }
            KeyCode::Char('b') | KeyCode::Backspace => {
                self.study_step_back();
            }
            KeyCode::Esc => {
                self.screen = Screen::LessonList;
                self.message = String::from("Select a lesson");
            }
            _ => {}
        }
    }

    fn start_lesson(&mut self) {
        let category = &STUDY_CATEGORIES[self.study_category];
        let lesson = &category.lessons[self.study_lesson];

        // Build all positions by replaying moves
        let mut pos = Position::from_fen(lesson.fen).unwrap_or_else(Position::start);
        let mut positions = vec![pos.clone()];

        for &(uci, _) in lesson.moves {
            if let Some(mv) = crate::board::Move::from_uci(uci) {
                pos = pos.make_move(mv);
                positions.push(pos.clone());
            }
        }

        self.study_positions = positions;
        self.study_move = 0;
        self.board = self.study_positions[0].clone();
        self.screen = Screen::LessonView;
        self.focus = Focus::Panel;

        self.message = format!("{} — n=next b=back Esc=list", lesson.title);
    }

    fn study_step_forward(&mut self) {
        let category = &STUDY_CATEGORIES[self.study_category];
        let lesson = &category.lessons[self.study_lesson];

        if self.study_move < lesson.moves.len() {
            self.study_move += 1;
            if self.study_move < self.study_positions.len() {
                self.board = self.study_positions[self.study_move].clone();
                self.play_sound(|a, s| a.play_move(s));
            }
        }
    }

    fn study_step_back(&mut self) {
        if self.study_move > 0 {
            self.study_move -= 1;
            self.board = self.study_positions[self.study_move].clone();
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
