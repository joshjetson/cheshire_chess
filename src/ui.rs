use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Widget};

use crate::app::{App, Screen};
use crate::board;
use crate::canvas::{CanvasMode, PIECE_TYPES, SHAPE_PALETTE};

// Cheshire Cat purple theme
const LIGHT_SQ: Color = Color::Rgb(200, 170, 220); // soft lavender
const DARK_SQ: Color = Color::Rgb(130, 80, 165);   // deep purple
const CURSOR_SQ: Color = Color::Rgb(255, 200, 255); // bright pink highlight
const TITLE_COLOR: Color = Color::Rgb(180, 120, 220);

// Board dimensions — 7 chars wide, 3 rows tall per square. DO NOT CHANGE.
const SQ_WIDTH: u16 = 7;
const SQ_HEIGHT: u16 = 3;

// Outlined piece art — 7 chars wide × 3 rows.
// Uses box-drawing and line characters to draw piece profiles/outlines.
fn piece_art(piece_type: usize) -> [&'static str; 3] {
    match piece_type {
        board::KING   => ["  ╺+╸  ",
                          " ╭─┼─╮ ",
                          " ╰───╯ "],

        board::QUEEN  => [" ╺╲│╱╸ ",
                          "  ╰▽╯  ",
                          " ╰───╯ "],

        board::ROOK   => [" ┌┐ ┌┐ ",
                          " │ ▐▌ │ ",
                          " └───┘ "],

        board::BISHOP => ["   ╱╲  ",
                          "  ╱◌╲  ",
                          " ╰───╯ "],

        board::KNIGHT => ["  ╱▔▔╲ ",
                          " ▕█▎ │ ",
                          " ╰───╯ "],

        board::PAWN   => ["  ╭─╮  ",
                          "  ╰─╯  ",
                          " ╰───╯ "],

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
    match app.screen {
        Screen::Menu => draw_menu(frame, app),
        Screen::ThemePicker => draw_theme_picker(frame, app),
        Screen::Analysis | Screen::Puzzle => draw_board_screen(frame, app),
        Screen::Results => draw_results(frame, app),
        Screen::Canvas => draw_canvas(frame, app),
        Screen::RoomBrowser => draw_room_browser(frame, app),
        Screen::RoomLobby => draw_room_lobby(frame, app),
        Screen::LiveGame => draw_live_game(frame, app),
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

fn draw_menu(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    let items: Vec<ListItem> = app
        .menu_items()
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let style = if i == app.menu_selection {
                Style::default()
                    .fg(Color::Rgb(255, 200, 255))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 180, 220))
            };
            let prefix = if i == app.menu_selection { " > " } else { "   " };
            ListItem::new(format!("{prefix}{item}")).style(style)
        })
        .collect();

    let puzzle_count = app.total_puzzles();
    let title = if puzzle_count > 0 {
        format!("Menu ({puzzle_count} puzzles indexed)")
    } else {
        String::from("Menu")
    };

    let menu = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title),
    );
    frame.render_widget(menu, chunks[1]);

    draw_status(frame, chunks[2], &app.message);
}

fn draw_theme_picker(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    let items: Vec<ListItem> = app
        .theme_counts()
        .iter()
        .enumerate()
        .map(|(i, (_tag, name, count))| {
            let style = if i == app.theme_selection {
                Style::default()
                    .fg(Color::Rgb(255, 200, 255))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 180, 220))
            };
            let prefix = if i == app.theme_selection { " > " } else { "   " };
            ListItem::new(format!("{prefix}{name} ({count})")).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Select Tactic Theme"),
    );
    frame.render_widget(list, chunks[1]);

    draw_status(frame, chunks[2], &app.message);
}

fn draw_results(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    let text = format!(
        "Session Complete!\n\nScore: {} / {}\n\nPress Enter to return to menu.",
        app.score_correct, app.score_total
    );
    let results = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Results"),
    );
    frame.render_widget(results, chunks[1]);

    draw_status(frame, chunks[2], &app.message);
}

fn draw_board_screen(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    // Board needs: 2 border + 2 rank label + 8*7 squares = 60 wide
    //              2 border + 8*3 squares + 1 file label row = 27 tall
    let board_width = 2 + 2 + 8 * SQ_WIDTH; // borders + rank label col + squares
    let board_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(board_width),
            Constraint::Min(0),
        ])
        .split(chunks[1]);

    let board_widget = BoardWidget {
        board: &app.board,
        cursor: app.cursor,
        custom_pieces: &app.custom_pieces,
        selected_sq: app.selected_sq,
        highlights: &app.highlights,
    };
    frame.render_widget(board_widget, board_area[0]);

    // Info panel
    let cursor_file = (b'a' + app.cursor % 8) as char;
    let cursor_rank = app.cursor / 8 + 1;
    let piece_info = match app.board.piece_at(app.cursor) {
        Some((pt, _color)) => piece_name(pt).to_string(),
        None => String::from("-"),
    };

    let mut info_lines = format!("  {cursor_file}{cursor_rank}  {piece_info}\n");

    if let Screen::Puzzle = app.screen {
        if let Some(puzzle) = app.puzzle_queue.get(app.puzzle_pos) {
            info_lines.push_str(&format!("\n  Puzzle: {}", puzzle.id));
            info_lines.push_str(&format!("\n  Rating: {}", puzzle.rating));
            info_lines.push_str(&format!(
                "\n  Themes: {}",
                puzzle.themes.join(", ")
            ));
            let solution = puzzle.solution_moves();
            info_lines.push_str(&format!("\n  Moves:  {}", solution.len()));
        }
    }

    let info = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Info"),
    );
    frame.render_widget(info, board_area[1]);

    draw_status(frame, chunks[2], &app.message);
}

const SELECTED_SQ: Color = Color::Rgb(120, 200, 120);  // green for selected piece
const HIGHLIGHT_SQ: Color = Color::Rgb(180, 220, 130); // yellow-green for legal targets

struct BoardWidget<'a> {
    board: &'a board::Position,
    cursor: u8,
    custom_pieces: &'a crate::canvas::CustomPieces,
    selected_sq: Option<u8>,
    highlights: &'a [u8],
}

impl Widget for BoardWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL);
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
                        // First pass: draw the piece
                        for (row, grid_row) in custom_grid.iter().enumerate() {
                            let line: String = grid_row.iter().collect();
                            buf.set_string(x, y_top + row as u16, &line, piece_style);
                        }
                        // Second pass: add ░ halo around filled cells
                        let halo_fg = match color {
                            board::Color::White => Color::Rgb(180, 170, 190),
                            board::Color::Black => Color::Rgb(70, 60, 80),
                        };
                        let halo_style = Style::default().fg(halo_fg).bg(bg);
                        for row in 0..3i16 {
                            for col in 0..7i16 {
                                if custom_grid[row as usize][col as usize] != ' ' {
                                    continue;
                                }
                                // Check if any neighbor is non-space
                                let has_neighbor = [(-1,0),(1,0),(0,-1),(0,1)]
                                    .iter()
                                    .any(|&(dr, dc)| {
                                        let nr = row + dr;
                                        let nc = col + dc;
                                        nr >= 0 && nr < 3 && nc >= 0 && nc < 7
                                            && custom_grid[nr as usize][nc as usize] != ' '
                                    });
                                if has_neighbor {
                                    buf.set_string(
                                        x + col as u16,
                                        y_top + row as u16,
                                        "░",
                                        halo_style,
                                    );
                                }
                            }
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

// ── Online Screens ─────────────────────────────────────────────────

fn draw_room_browser(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    let items: Vec<ListItem> = if app.room_list.is_empty() {
        vec![ListItem::new("  No rooms yet. Press [n] to create one.").style(
            Style::default().fg(Color::Rgb(160, 140, 180)),
        )]
    } else {
        app.room_list.iter().enumerate().map(|(i, room)| {
            let style = if i == app.room_selection {
                Style::default().fg(Color::Rgb(255, 200, 255)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 180, 220))
            };
            let prefix = if i == app.room_selection { " > " } else { "   " };
            let games = if room.active_games > 0 { format!(" [{} game(s)]", room.active_games) } else { String::new() };
            ListItem::new(format!("{prefix}{} ({} players, {} tables){games}", room.name, room.player_count, room.table_count)).style(style)
        }).collect()
    };

    let list = List::new(items).block(
        Block::default().borders(Borders::ALL)
            .title(format!("Game Rooms — [n]ew room, [r]efresh, Enter=join, Esc=back")),
    );
    frame.render_widget(list, chunks[1]);
    draw_status(frame, chunks[2], &app.message);
}

fn draw_room_lobby(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Left: tables + players
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(cols[0]);

    let room_name = app.current_room.as_ref().map(|r| r.name.as_str()).unwrap_or("Room");

    // Tables list
    let table_items: Vec<ListItem> = if app.tables.is_empty() {
        vec![ListItem::new("  No tables. Press [t] to create one.").style(
            Style::default().fg(Color::Rgb(160, 140, 180))
        )]
    } else {
        app.tables.iter().enumerate().map(|(i, t)| {
            let style = if i == app.table_selection {
                Style::default().fg(Color::Rgb(255, 200, 255)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 180, 220))
            };
            let prefix = if i == app.table_selection { " > " } else { "   " };
            let white = t.white.as_ref().map(|p| p.name.as_str()).unwrap_or("(open)");
            let black = t.black.as_ref().map(|p| p.name.as_str()).unwrap_or("(open)");
            let status = if t.has_game { "playing" } else if t.white.is_some() || t.black.is_some() { "waiting" } else { "empty" };
            let spectators = if t.spectator_count > 0 { format!(" +{} watching", t.spectator_count) } else { String::new() };
            ListItem::new(format!("{prefix}Table {}: {white} vs {black} [{status}]{spectators}", t.id)).style(style)
        }).collect()
    };

    let tables = List::new(table_items).block(
        Block::default().borders(Borders::ALL)
            .title(format!("{room_name} — [t]able, Enter=join")),
    );
    frame.render_widget(tables, left[0]);

    // Players list
    let player_items: Vec<ListItem> = app.room_players.iter().map(|p| {
        let status = match p.status {
            crate::protocol::PlayerStatus::Idle => "",
            crate::protocol::PlayerStatus::Playing => " [playing]",
            crate::protocol::PlayerStatus::Spectating => " [watching]",
        };
        let me = if Some(p.id) == app.my_id { " (you)" } else { "" };
        ListItem::new(format!("   {}{me}{status}", p.name))
            .style(Style::default().fg(Color::Rgb(180, 160, 200)))
    }).collect();

    let plist = List::new(player_items).block(
        Block::default().borders(Borders::ALL)
            .title(format!("{} players", app.room_players.len())),
    );
    frame.render_widget(plist, left[1]);

    // Right: chat
    draw_chat(frame, app, cols[1]);

    draw_status(frame, chunks[2], &app.message);
}

fn draw_live_game(frame: &mut Frame, app: &App) {
    let chunks = standard_layout(frame);
    draw_title(frame, chunks[0]);

    let board_width = 2 + 2 + 8 * SQ_WIDTH;
    let game_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(board_width), Constraint::Min(0)])
        .split(chunks[1]);

    let board_widget = BoardWidget {
        board: &app.board,
        cursor: app.cursor,
        custom_pieces: &app.custom_pieces,
        selected_sq: app.selected_sq,
        highlights: &app.highlights,
    };
    frame.render_widget(board_widget, game_area[0]);

    // Right side: chat
    draw_chat(frame, app, game_area[1]);

    draw_status(frame, chunks[2], &app.message);
}

fn draw_chat(frame: &mut Frame, app: &App, area: Rect) {
    let chat_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Messages
    let msg_count = app.chat.messages.len();
    let visible = chat_layout[0].height.saturating_sub(2) as usize;
    let skip = if msg_count > visible { msg_count - visible } else { 0 };

    let items: Vec<ListItem> = app.chat.messages.iter().skip(skip).map(|(sender, body, kind)| {
        let style = match kind {
            crate::protocol::ChatKind::System => Style::default().fg(Color::Rgb(120, 120, 140)),
            crate::protocol::ChatKind::Player => Style::default().fg(Color::Rgb(200, 180, 220)),
            crate::protocol::ChatKind::Spectator => Style::default().fg(Color::Rgb(150, 170, 200)),
        };
        let text = if sender.is_empty() {
            format!("  {body}")
        } else {
            format!("  {sender}: {body}")
        };
        ListItem::new(text).style(style)
    }).collect();

    let chat = List::new(items).block(
        Block::default().borders(Borders::ALL).title("Chat"),
    );
    frame.render_widget(chat, chat_layout[0]);

    // Input
    let input_style = if app.chat.typing {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::Rgb(100, 90, 110))
    };
    let input_text = if app.chat.typing {
        format!("> {}_", app.chat.input)
    } else {
        String::from("  Tab to chat...")
    };
    let input = Paragraph::new(input_text)
        .style(input_style)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, chat_layout[1]);
}
