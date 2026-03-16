use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Widget};

use crate::app::{App, Screen};
use crate::board;
use crate::canvas::{CanvasMode, PIECE_TYPES, SHAPE_PALETTE};
use crate::settings::{SETTINGS_ITEMS, SOUND_EVENT_NAMES, SYNTH_PARAM_NAMES};

// Cheshire Cat purple theme
const LIGHT_SQ: Color = Color::Rgb(200, 170, 220); // soft lavender
const DARK_SQ: Color = Color::Rgb(130, 80, 165);   // deep purple
const CURSOR_SQ: Color = Color::Rgb(255, 200, 255); // bright pink highlight
const TITLE_COLOR: Color = Color::Rgb(180, 120, 220);

// Board dimensions — 7 chars wide, 3 rows tall per square. DO NOT CHANGE.
const SQ_WIDTH: u16 = 7;
const SQ_HEIGHT: u16 = 3;

// Default piece art — matches the custom pieces from custom_pieces.txt.
// These are the built-in defaults so installed copies look identical.
fn piece_art(piece_type: usize) -> [&'static str; 3] {
    match piece_type {
        board::KING   => ["       ",
                          " ▕▟✚▙▏ ",
                          "  ▀▀▀  "],

        board::QUEEN  => ["       ",
                          " ▕▟✠▙▏ ",
                          "  ◥■◤  "],

        board::ROOK   => ["       ",
                          " ▕▟▆▙▏ ",
                          "  ▀▀▀  "],

        board::BISHOP => ["   ▲   ",
                          "  ▐▀▌  ",
                          "       "],

        board::KNIGHT => [" ▂▅▅▅▃▃",
                          "▕▣▞ ▚▚▚",
                          " ▀   ▚▚"],

        board::PAWN   => ["   ⭘   ",
                          "  ▜█▛  ",
                          "       "],

        _ =>             ["       ",
                          "       ",
                          "       "],
    }
}

// For the info panel
fn piece_name(piece_type: usize) -> &'static str {
    match piece_type {
        board::PAWN => "Pawn",
        board::KNIGHT => "Knight",
        board::BISHOP => "Bishop",
        board::ROOK => "Rook",
        board::QUEEN => "Queen",
        board::KING => "King",
        _ => "?",
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    // Canvas and shape picker are full-screen (no board)
    match app.screen {
        Screen::Canvas => { draw_canvas(frame, app); return; }
        _ => {}
    }

    // Adaptive layout: shrink/hide title+status when terminal is small
    let total_height = frame.area().height;
    let (title_h, status_h) = if total_height >= 33 {
        (3u16, 3u16) // full: title + status bars
    } else if total_height >= 29 {
        (1, 1) // compact: single-line title + status
    } else {
        (0, 1) // minimal: no title, single-line status
    };

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(title_h), Constraint::Min(0), Constraint::Length(status_h)])
        .split(frame.area());

    if title_h > 0 {
        if title_h >= 3 {
            draw_title(frame, outer[0]);
        } else {
            let title = Paragraph::new(" Cheshire Chess")
                .style(Style::default().fg(TITLE_COLOR).add_modifier(Modifier::BOLD));
            frame.render_widget(title, outer[0]);
        }
    }
    if status_h >= 3 {
        draw_status(frame, outer[2], &app.message);
    } else {
        let status = Paragraph::new(format!(" {}", app.message))
            .style(Style::default().fg(Color::Rgb(160, 140, 180)));
        frame.render_widget(status, outer[2]);
    }

    let board_width = 2 + 2 + 8 * SQ_WIDTH; // borders + rank label + squares
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(board_width), Constraint::Min(0)])
        .split(outer[1]);

    // Left: always the board
    let board_widget = BoardWidget {
        board: &app.board,
        cursor: app.cursor,
        custom_pieces: &app.custom_pieces,
        selected_sq: app.selected_sq,
        highlights: &app.highlights,
        focused: app.focus == crate::app::Focus::Board,
    };
    frame.render_widget(board_widget, main[0]);

    // Right: context-dependent pane
    match app.screen {
        Screen::Menu | Screen::Analysis => draw_right_menu(frame, app, main[1]),
        Screen::ThemePicker => draw_right_theme_picker(frame, app, main[1]),
        Screen::Puzzle => draw_right_puzzle(frame, app, main[1]),
        Screen::Results => draw_right_results(frame, app, main[1]),
        Screen::RoomBrowser => draw_right_room_browser(frame, app, main[1]),
        Screen::RoomNameInput => draw_right_room_name_input(frame, app, main[1]),
        Screen::RoomLobby => draw_right_room_lobby(frame, app, main[1]),
        Screen::LiveGame => draw_right_live_game(frame, app, main[1]),
        Screen::Settings => draw_right_settings(frame, app, main[1]),
        Screen::SoundSettings => draw_right_sound_settings(frame, app, main[1]),
        Screen::SoundEventEdit => draw_right_sound_event_edit(frame, app, main[1]),
        Screen::NameEdit => draw_right_name_edit(frame, app, main[1]),
        Screen::Canvas => {} // handled above
    }
}

fn draw_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("  Cheshire Chess")
        .style(Style::default().fg(TITLE_COLOR).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
}

fn draw_status(frame: &mut Frame, area: Rect, message: &str) {
    let status = Paragraph::new(message)
        .style(Style::default().fg(Color::Rgb(160, 140, 180)))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, area);
}

fn standard_layout(frame: &mut Frame) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area())
        .to_vec()
}

const SELECTED_SQ: Color = Color::Rgb(120, 200, 120);  // green for selected piece
const HIGHLIGHT_SQ: Color = Color::Rgb(180, 220, 130); // yellow-green for legal targets

struct BoardWidget<'a> {
    board: &'a board::Position,
    cursor: u8,
    custom_pieces: &'a crate::canvas::CustomPieces,
    selected_sq: Option<u8>,
    highlights: &'a [u8],
    focused: bool,
}

impl Widget for BoardWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            Style::default().fg(Color::Rgb(200, 170, 230))
        } else {
            Style::default().fg(Color::Rgb(80, 60, 100))
        };
        let block = Block::default().borders(Borders::ALL).border_style(border_style);
        let inner = block.inner(area);
        block.render(area, buf);

        let label_col_width = 2u16;

        for display_rank in 0..8u8 {
            let rank = 7 - display_rank;
            let y_top = inner.y + display_rank as u16 * SQ_HEIGHT;

            if y_top + SQ_HEIGHT > inner.y + inner.height {
                break;
            }

            // Rank label centered vertically
            buf.set_string(
                inner.x,
                y_top + 1,
                format!("{} ", rank + 1),
                Style::default().fg(Color::Rgb(160, 140, 180)),
            );

            for file in 0..8u8 {
                let sq = rank * 8 + file;
                let x = inner.x + label_col_width + file as u16 * SQ_WIDTH;

                if x + SQ_WIDTH > inner.x + inner.width {
                    break;
                }

                let is_light = (rank + file) % 2 == 1;
                let is_cursor = sq == self.cursor;
                let is_selected = self.selected_sq == Some(sq);
                let is_highlight = self.highlights.contains(&sq);

                let bg = if is_cursor {
                    CURSOR_SQ
                } else if is_selected {
                    SELECTED_SQ
                } else if is_highlight {
                    HIGHLIGHT_SQ
                } else if is_light {
                    LIGHT_SQ
                } else {
                    DARK_SQ
                };

                // Fill all 3 rows with square background
                let blank = " ".repeat(SQ_WIDTH as usize);
                let bg_style = Style::default().bg(bg);
                buf.set_string(x, y_top, &blank, bg_style);
                buf.set_string(x, y_top + 1, &blank, bg_style);
                buf.set_string(x, y_top + 2, &blank, bg_style);

                // Draw piece — custom art if available, else built-in outline
                if let Some((pt, color)) = self.board.piece_at(sq) {
                    let fg = match color {
                        board::Color::White => Color::Rgb(255, 255, 255),
                        board::Color::Black => Color::Rgb(40, 30, 45),
                    };
                    let piece_style = Style::default().fg(fg).bg(bg);

                    if let Some(custom_grid) = self.custom_pieces.get(pt) {
                        for (row, grid_row) in custom_grid.iter().enumerate() {
                            let line: String = grid_row.iter().collect();
                            buf.set_string(x, y_top + row as u16, &line, piece_style);
                        }
                    } else {
                        let art = piece_art(pt);
                        for (row, art_row) in art.iter().enumerate() {
                            buf.set_string(x, y_top + row as u16, art_row, piece_style);
                        }
                    }
                }
            }
        }

        // File labels
        let label_y = inner.y + 8 * SQ_HEIGHT;
        if label_y < inner.y + inner.height {
            for file in 0..8u8 {
                let x = inner.x + label_col_width + file as u16 * SQ_WIDTH + SQ_WIDTH / 2;
                if x < inner.x + inner.width {
                    let label = (b'a' + file) as char;
                    buf.set_string(
                        x,
                        label_y,
                        format!("{label}"),
                        Style::default().fg(Color::Rgb(160, 140, 180)),
                    );
                }
            }
        }
    }
}

fn draw_canvas(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    match app.canvas.mode {
        CanvasMode::PiecePicker => draw_canvas_piece_picker(frame, app, chunks[1]),
        CanvasMode::Drawing => draw_canvas_editor(frame, app, chunks[1]),
        CanvasMode::ShapePicker => draw_canvas_shape_picker(frame, app, chunks[1]),
    }

    draw_status(frame, chunks[2], &app.message);
}

fn draw_canvas_piece_picker(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = PIECE_TYPES
        .iter()
        .enumerate()
        .map(|(i, &(_pt, name))| {
            let style = if i == app.canvas.selected_piece {
                Style::default()
                    .fg(Color::Rgb(255, 200, 255))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 180, 220))
            };
            let prefix = if i == app.canvas.selected_piece { " > " } else { "   " };
            let saved = if app.custom_pieces.get(PIECE_TYPES[i].0).is_some() {
                " [saved]"
            } else {
                ""
            };
            ListItem::new(format!("{prefix}{name}{saved}")).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Piece Canvas — Select a piece to draw"),
    );
    frame.render_widget(list, area);
}

fn draw_canvas_editor(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),
            Constraint::Min(0),
        ])
        .split(area);

    // Left: the drawing grid (zoomed in — each cell is 3 chars wide × 1 row)
    let grid_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Drawing: {}", app.canvas.piece_name()));
    let grid_inner = grid_block.inner(cols[0]);
    frame.render_widget(grid_block, cols[0]);

    let cell_width = 3u16;

    // Draw grid cells
    for row in 0..3usize {
        for col in 0..7usize {
            let gx = grid_inner.x + col as u16 * cell_width;
            let gy = grid_inner.y + row as u16 * 2; // 2 rows per grid row for visibility

            if gy + 1 >= grid_inner.y + grid_inner.height {
                break;
            }

            let ch = app.canvas.grid[row][col];
            let is_cursor = row == app.canvas.cursor_row && col == app.canvas.cursor_col;

            let bg = if is_cursor {
                Color::Rgb(100, 80, 140)
            } else {
                Color::Rgb(50, 40, 60)
            };

            let cell_str = if ch == ' ' {
                "   ".to_string()
            } else {
                format!(" {ch} ")
            };

            // Top row: the character
            let style = Style::default().fg(Color::White).bg(bg);
            frame.buffer_mut().set_string(gx, gy, &cell_str, style);
            // Bottom row: border
            let border_str = if is_cursor { "───" } else { "···" };
            frame.buffer_mut().set_string(
                gx,
                gy + 1,
                border_str,
                Style::default().fg(Color::Rgb(80, 60, 100)).bg(Color::Rgb(30, 25, 35)),
            );
        }
    }

    // Preview section: show the piece at actual size on both square colors
    let preview_y = grid_inner.y + 8; // below the grid
    if preview_y + 4 < grid_inner.y + grid_inner.height {
        frame.buffer_mut().set_string(
            grid_inner.x,
            preview_y,
            "Preview:",
            Style::default().fg(Color::Rgb(160, 140, 180)),
        );

        // White piece on dark square
        for (r, grid_row) in app.canvas.grid.iter().enumerate() {
            let s: String = grid_row.iter().collect();
            frame.buffer_mut().set_string(
                grid_inner.x,
                preview_y + 1 + r as u16,
                &s,
                Style::default().fg(Color::White).bg(DARK_SQ),
            );
            frame.buffer_mut().set_string(
                grid_inner.x + 8,
                preview_y + 1 + r as u16,
                &s,
                Style::default().fg(Color::Rgb(40, 30, 45)).bg(LIGHT_SQ),
            );
        }
    }

    // Right panel: current shape + instructions
    let right_text = format!(
        "Current shape: {}\n\n\
         Controls:\n\
         hjkl/arrows - move cursor\n\
         Enter       - place shape\n\
         Space/Del   - erase cell\n\
         Tab         - pick shape\n\
         [s]         - save piece\n\
         [c]         - clear grid\n\
         Esc         - back to list",
        app.canvas.current_char()
    );
    let right = Paragraph::new(right_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Tools"),
    );
    frame.render_widget(right, cols[1]);
}

fn draw_canvas_shape_picker(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            "Select Shape ({}/{}) — arrows to move, Enter to pick, Esc to cancel",
            app.canvas.current_shape + 1,
            SHAPE_PALETTE.len()
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cell_w = 4u16;
    let cols_per_row = ((inner.width / cell_w) as usize).max(1);
    let visible_rows = inner.height as usize;

    // Scroll so selected row is always visible
    let selected_row = app.canvas.current_shape / cols_per_row;
    let scroll = if selected_row >= visible_rows {
        selected_row - visible_rows + 1
    } else {
        0
    };

    for (i, &ch) in SHAPE_PALETTE.iter().enumerate() {
        let row = i / cols_per_row;
        let col = i % cols_per_row;

        if row < scroll { continue; }
        let display_row = row - scroll;

        let px = inner.x + col as u16 * cell_w;
        let py = inner.y + display_row as u16;

        if py >= inner.y + inner.height || px + cell_w > inner.x + inner.width {
            continue;
        }

        let is_selected = i == app.canvas.current_shape;
        let style = if is_selected {
            Style::default()
                .fg(Color::Rgb(255, 200, 255))
                .bg(Color::Rgb(80, 50, 100))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if is_selected { ">" } else { " " };
        frame.buffer_mut().set_string(px, py, format!("{prefix}{ch} "), style);
    }
}

// ── Right Pane Functions ───────────────────────────────────────────

fn list_style(selected: bool) -> Style {
    if selected {
        Style::default().fg(Color::Rgb(255, 200, 255)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(200, 180, 220))
    }
}

fn prefix(selected: bool) -> &'static str {
    if selected { " > " } else { "   " }
}

fn draw_right_menu(frame: &mut Frame, app: &App, area: Rect) {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(area);

    let items: Vec<ListItem> = app.menu_items().iter().enumerate().map(|(i, &item)| {
        ListItem::new(format!("{}{item}", prefix(i == app.menu_selection)))
            .style(list_style(i == app.menu_selection))
    }).collect();

    let puzzle_count = app.total_puzzles();
    let title = if puzzle_count > 0 { format!("Menu ({puzzle_count} puzzles)") } else { String::from("Menu") };
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, split[0]);

    // Cursor info
    let cursor_file = (b'a' + app.cursor % 8) as char;
    let cursor_rank = app.cursor / 8 + 1;
    let piece_info = match app.board.piece_at(app.cursor) {
        Some((pt, _)) => piece_name(pt).to_string(),
        None => String::from("-"),
    };
    let info = Paragraph::new(format!("  {cursor_file}{cursor_rank}  {piece_info}"))
        .block(Block::default().borders(Borders::ALL).title("Board"));
    frame.render_widget(info, split[1]);
}

fn draw_right_theme_picker(frame: &mut Frame, app: &App, area: Rect) {
    let theme_counts = app.theme_counts();
    let items: Vec<ListItem> = if !theme_counts.is_empty() {
        theme_counts.iter().enumerate().map(|(i, (_tag, name, count))| {
            ListItem::new(format!("{}{name} ({count})", prefix(i == app.theme_selection)))
                .style(list_style(i == app.theme_selection))
        }).collect()
    } else {
        // No local index — show theme names without counts (server will provide puzzles)
        crate::puzzle::TACTIC_THEMES.iter().enumerate().map(|(i, &(_tag, name))| {
            ListItem::new(format!("{}{name}", prefix(i == app.theme_selection)))
                .style(list_style(i == app.theme_selection))
        }).collect()
    };
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Select Tactic Theme"));
    frame.render_widget(list, area);
}

fn draw_right_puzzle(frame: &mut Frame, app: &App, area: Rect) {
    let cursor_file = (b'a' + app.cursor % 8) as char;
    let cursor_rank = app.cursor / 8 + 1;
    let piece_info = match app.board.piece_at(app.cursor) {
        Some((pt, _)) => piece_name(pt).to_string(),
        None => String::from("-"),
    };
    let mut lines = format!("  {cursor_file}{cursor_rank}  {piece_info}\n");
    if let Some(puzzle) = app.puzzle_queue.get(app.puzzle_pos) {
        lines.push_str(&format!("\n  Puzzle: {}", puzzle.id));
        lines.push_str(&format!("\n  Rating: {}", puzzle.rating));
        lines.push_str(&format!("\n  Themes: {}", puzzle.themes.join(", ")));
        lines.push_str(&format!("\n  Moves:  {}", puzzle.solution_moves().len()));
        lines.push_str(&format!("\n\n  {}/{}", app.puzzle_pos + 1, app.puzzle_queue.len()));
        lines.push_str(&format!("\n  Score: {}/{}", app.score_correct, app.score_total));
    }
    let info = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Puzzle"));
    frame.render_widget(info, area);
}

fn draw_right_results(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        "\n  Session Complete!\n\n  Score: {} / {}\n\n  Press Enter to return.",
        app.score_correct, app.score_total
    );
    let para = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Results"));
    frame.render_widget(para, area);
}

fn draw_right_room_browser(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = if app.room_list.is_empty() {
        vec![ListItem::new("  No rooms yet. Press [n] to create one.").style(Style::default().fg(Color::Rgb(160, 140, 180)))]
    } else {
        app.room_list.iter().enumerate().map(|(i, room)| {
            let games = if room.active_games > 0 { format!(" [{}g]", room.active_games) } else { String::new() };
            ListItem::new(format!("{}{} ({} players, {} tables){games}",
                prefix(i == app.room_selection), room.name, room.player_count, room.table_count))
                .style(list_style(i == app.room_selection))
        }).collect()
    };
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Game Rooms"));
    frame.render_widget(list, area);
}

fn draw_right_room_lobby(frame: &mut Frame, app: &App, area: Rect) {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(30), Constraint::Percentage(30)])
        .split(area);

    // Tables
    let room_name = app.current_room.as_ref().map(|r| r.name.as_str()).unwrap_or("Room");
    let table_items: Vec<ListItem> = if app.tables.is_empty() {
        vec![ListItem::new("  No tables. [t] to create.").style(Style::default().fg(Color::Rgb(160, 140, 180)))]
    } else {
        app.tables.iter().enumerate().map(|(i, t)| {
            let w = t.white.as_ref().map(|p| p.name.as_str()).unwrap_or("?");
            let b = t.black.as_ref().map(|p| p.name.as_str()).unwrap_or("?");
            let st = if t.has_game { "playing" } else { "waiting" };
            ListItem::new(format!("{}{w} v {b} [{st}]", prefix(i == app.table_selection)))
                .style(list_style(i == app.table_selection))
        }).collect()
    };
    frame.render_widget(List::new(table_items).block(Block::default().borders(Borders::ALL).title(format!("{room_name} [t]able"))), split[0]);

    // Players
    let player_items: Vec<ListItem> = app.room_players.iter().map(|p| {
        let me = if Some(p.id) == app.my_id { " (you)" } else { "" };
        ListItem::new(format!("   {}{me}", p.name)).style(Style::default().fg(Color::Rgb(180, 160, 200)))
    }).collect();
    frame.render_widget(List::new(player_items).block(Block::default().borders(Borders::ALL).title("Players")), split[1]);

    // Chat
    draw_chat_pane(frame, app, split[2]);
}

fn draw_right_live_game(frame: &mut Frame, app: &App, area: Rect) {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    // Game info panel
    let my_id = app.my_id;
    let is_white = app.live_white == my_id;
    let is_black = app.live_black == my_id;
    let is_spectator = !is_white && !is_black;

    let my_role = if is_white { "White" } else if is_black { "Black" } else { "Spectating" };
    let turn = match app.board.side_to_move {
        crate::board::Color::White => "White to move",
        crate::board::Color::Black => "Black to move",
    };
    let your_turn = app.game_active && ((is_white && app.board.side_to_move == crate::board::Color::White)
        || (is_black && app.board.side_to_move == crate::board::Color::Black));

    let status = if !app.game_active {
        "Game over"
    } else if your_turn {
        "YOUR MOVE"
    } else if is_spectator {
        turn
    } else {
        "Waiting..."
    };

    let in_check = app.game_active && app.board.in_check(app.board.side_to_move);
    let check_str = if in_check { " CHECK!" } else { "" };

    let info_text = format!(
        "\n  You: {my_role}\n  {turn}{check_str}\n\n  {status}\n\n  Tab=chat r=resign Esc=leave"
    );

    let info_style = if your_turn {
        Style::default().fg(Color::Rgb(255, 220, 150))
    } else {
        Style::default().fg(Color::Rgb(200, 180, 220))
    };

    frame.render_widget(
        Paragraph::new(info_text).style(info_style)
            .block(Block::default().borders(Borders::ALL).title("Game")),
        split[0],
    );

    // Chat below
    draw_chat_pane(frame, app, split[1]);
}

fn draw_chat_pane(frame: &mut Frame, app: &App, area: Rect) {
    let chat_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let msg_count = app.chat.messages.len();
    let visible = chat_split[0].height.saturating_sub(2) as usize;
    let skip = if msg_count > visible { msg_count - visible } else { 0 };

    let items: Vec<ListItem> = app.chat.messages.iter().skip(skip).map(|(sender, body, kind)| {
        let style = match kind {
            crate::protocol::ChatKind::System => Style::default().fg(Color::Rgb(120, 120, 140)),
            crate::protocol::ChatKind::Player => Style::default().fg(Color::Rgb(200, 180, 220)),
            crate::protocol::ChatKind::Spectator => Style::default().fg(Color::Rgb(150, 170, 200)),
        };
        let text = if sender.is_empty() { format!("  {body}") } else { format!("  {sender}: {body}") };
        ListItem::new(text).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).title("Chat")), chat_split[0]);

    let input_text = if app.chat.typing { format!("> {}_", app.chat.input) } else { String::from("  Tab to chat...") };
    let input_style = if app.chat.typing { Style::default().fg(Color::White) } else { Style::default().fg(Color::Rgb(100, 90, 110)) };
    frame.render_widget(Paragraph::new(input_text).style(input_style).block(Block::default().borders(Borders::ALL)), chat_split[1]);
}

fn draw_right_settings(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = SETTINGS_ITEMS.iter().enumerate().map(|(i, &item)| {
        let detail = match i {
            0 => format!("  [{}]", app.settings.player_name),
            1 => {
                if app.audio.is_none() {
                    String::from("  [unavailable — install audio deps]")
                } else {
                    format!("  [{}]", if app.settings.sound.enabled { "on" } else { "off" })
                }
            }
            _ => String::new(),
        };
        ListItem::new(format!("{}{item}{detail}", prefix(i == app.settings_selection)))
            .style(list_style(i == app.settings_selection))
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).title("Settings")), area);
}

fn draw_right_name_edit(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!("\n  Name:\n\n  > {}_\n\n  Enter=save Esc=cancel", app.name_input);
    frame.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Edit Name")), area);
}

fn draw_right_sound_settings(frame: &mut Frame, app: &App, area: Rect) {
    if app.audio.is_none() {
        let text = "\n  Audio not available.\n\n  On Linux, install:\n    apt install libasound2-dev\n\n  Then reinstall:\n    cargo install cheshire_chess";
        frame.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Sound")), area);
        return;
    }
    let items: Vec<ListItem> = SOUND_EVENT_NAMES.iter().enumerate().map(|(i, &name)| {
        let p = app.get_event_params(i);
        ListItem::new(format!("{}{name} ({} {}Hz)", prefix(i == app.sound_event_selection), p.waveform.name(), p.frequency as u32))
            .style(list_style(i == app.sound_event_selection))
    }).collect();
    let mute = if app.settings.sound.enabled { "" } else { " [MUTED]" };
    frame.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).title(format!("Sound{mute} [m]ute Enter=edit"))), area);
}

fn draw_right_sound_event_edit(frame: &mut Frame, app: &App, area: Rect) {
    let params = app.get_event_params(app.sound_event_selection);
    let event_name = SOUND_EVENT_NAMES[app.sound_event_selection];
    let values = [
        params.waveform.name().to_string(), format!("{:.0}Hz", params.frequency),
        format!("{:.3}s", params.attack), format!("{:.3}s", params.decay),
        format!("{:.2}", params.sustain), format!("{:.3}s", params.release),
        format!("{:.2}", params.volume), format!("{:.1}Hz", params.lfo_rate),
        format!("{:.2}", params.lfo_depth), format!("{}ms", params.duration_ms),
    ];
    let items: Vec<ListItem> = SYNTH_PARAM_NAMES.iter().enumerate().map(|(i, &name)| {
        let sel = i == app.sound_param_selection;
        let arrow = if sel { " < > " } else { "     " };
        ListItem::new(format!("{}{name:<10} {:<8}{arrow}", prefix(sel), values[i])).style(list_style(sel))
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).title(format!("{event_name} [p]review [s]ave"))), area);
}

fn draw_right_room_name_input(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!("\n  Room Name:\n\n  > {}_\n\n  Enter=create Esc=cancel", app.room_name_input);
    frame.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("New Room")), area);
}
