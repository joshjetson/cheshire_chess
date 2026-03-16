use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::board;

/// The 7×3 grid for one piece. Each cell is a char (space = empty).
pub type PieceGrid = [[char; 7]; 3];

const EMPTY_GRID: PieceGrid = [[' '; 7]; 3];

/// All 6 custom piece designs.
#[derive(Clone)]
pub struct CustomPieces {
    pub pieces: HashMap<usize, PieceGrid>,
    save_path: PathBuf,
}

/// Characters the user can pick from to draw with.
pub const SHAPE_PALETTE: &[char] = &[
    // Block Elements
    '█', '▀', '▁', '▂', '▃', '▄', '▅', '▆', '▇',
    '▉', '▊', '▋', '▌', '▍', '▎', '▏', '▐',
    '░', '▒', '▓', '▔', '▕',
    '▖', '▗', '▘', '▙', '▚', '▛', '▜', '▝', '▞', '▟',
    // Box Drawing - Single
    '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼',
    // Box Drawing - Double
    '═', '║', '╔', '╗', '╚', '╝', '╠', '╣', '╦', '╩', '╬',
    // Box Drawing - Rounded
    '╭', '╮', '╯', '╰',
    // Box Drawing - Heavy
    '━', '┃', '┏', '┓', '┗', '┛', '┣', '┫', '┳', '┻', '╋',
    // Box Drawing - Dashes
    '┄', '┅', '┆', '┇', '┈', '┉', '┊', '┋', '╌', '╍', '╎', '╏',
    // Diagonals
    '╱', '╲',
    // Geometric Shapes - Squares
    '■', '□', '▢', '▣', '▤', '▥', '▦', '▧', '▨', '▩', '▪', '▫', '▬', '▭', '▮', '▯',
    // Geometric Shapes - Triangles
    '▰', '▱', '▲', '△', '▴', '▵', '▶', '▷', '▸', '▹',
    '▼', '▽', '▾', '▿', '◀', '◁', '◂', '◃',
    // Geometric Shapes - Diamonds & Circles
    '◆', '◇', '◈', '◉', '◊', '○', '◌', '◍', '◎', '●',
    '◐', '◑', '◒', '◓', '◔', '◕', '◖', '◗',
    '◘', '◙', '◚', '◛', '◜', '◝', '◞', '◟', '◠', '◡',
    // Geometric Shapes - Corner triangles
    '◢', '◣', '◤', '◥',
    // Geometric Shapes - More
    '◦', '◧', '◨', '◩', '◪', '◫', '◬', '◭', '◮', '◯',
    '◰', '◱', '◲', '◳', '◴', '◵', '◶', '◷',
    '◸', '◹', '◺', '◻', '◼', '◽', '◾', '◿',
    // Stars & Crosses
    '★', '☆', '✦', '✧', '✚', '✛', '✜', '✝',
    '⊕', '⊖', '⊗', '⊘', '⊙', '⊞', '⊟', '⊠', '⊡',
    // Chess pieces (for reference/mixing in)
    '♔', '♕', '♖', '♗', '♘', '♙', '♚', '♛', '♜', '♝', '♞', '♟',
    // Misc useful
    '+', '*', '^', '~', '·', '•', '∘', '∙',
];

pub const PIECE_TYPES: &[(usize, &str)] = &[
    (board::KING, "King"),
    (board::QUEEN, "Queen"),
    (board::ROOK, "Rook"),
    (board::BISHOP, "Bishop"),
    (board::KNIGHT, "Knight"),
    (board::PAWN, "Pawn"),
];

impl CustomPieces {
    pub fn new(save_path: PathBuf) -> Self {
        let mut cp = Self {
            pieces: HashMap::new(),
            save_path,
        };
        cp.load();
        cp
    }

    pub fn get(&self, piece_type: usize) -> Option<&PieceGrid> {
        self.pieces.get(&piece_type)
    }

    pub fn set(&mut self, piece_type: usize, grid: PieceGrid) {
        self.pieces.insert(piece_type, grid);
    }

    /// Save all pieces to a simple text file.
    /// Format: piece_type_index followed by 3 lines of 7 chars each, separated by blank lines.
    pub fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.save_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = String::new();
        for &(pt, name) in PIECE_TYPES {
            if let Some(grid) = self.pieces.get(&pt) {
                out.push_str(&format!("# {name}\n"));
                out.push_str(&format!("{pt}\n"));
                for row in grid {
                    let line: String = row.iter().collect();
                    out.push_str(&line);
                    out.push('\n');
                }
                out.push('\n');
            }
        }
        fs::write(&self.save_path, out)
    }

    /// Load pieces from the save file.
    fn load(&mut self) {
        let content = match fs::read_to_string(&self.save_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut lines = content.lines().peekable();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let pt: usize = match line.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let mut grid = EMPTY_GRID;
            for row in &mut grid {
                if let Some(row_line) = lines.next() {
                    for (col, ch) in row_line.chars().enumerate() {
                        if col < 7 {
                            row[col] = ch;
                        }
                    }
                }
            }
            self.pieces.insert(pt, grid);
        }
    }
}

/// State for the canvas editor screen.
pub struct CanvasState {
    pub selected_piece: usize,     // index into PIECE_TYPES
    pub cursor_row: usize,         // 0..2
    pub cursor_col: usize,         // 0..6
    pub current_shape: usize,      // index into SHAPE_PALETTE
    pub grid: PieceGrid,           // working grid being edited
    pub mode: CanvasMode,
}

pub enum CanvasMode {
    PiecePicker,  // selecting which piece to edit
    Drawing,      // editing the 7×3 grid
    ShapePicker,  // selecting a shape from the palette
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            selected_piece: 0,
            cursor_row: 1,
            cursor_col: 3,
            current_shape: 0,
            grid: EMPTY_GRID,
            mode: CanvasMode::PiecePicker,
        }
    }

    pub fn piece_type(&self) -> usize {
        PIECE_TYPES[self.selected_piece].0
    }

    pub fn piece_name(&self) -> &str {
        PIECE_TYPES[self.selected_piece].1
    }

    pub fn current_char(&self) -> char {
        SHAPE_PALETTE[self.current_shape]
    }

    /// Load the current piece's grid from custom pieces (or blank if none).
    pub fn load_grid(&mut self, customs: &CustomPieces) {
        self.grid = customs
            .get(self.piece_type())
            .copied()
            .unwrap_or(EMPTY_GRID);
    }

    /// Place the current shape at cursor position.
    pub fn stamp(&mut self) {
        self.grid[self.cursor_row][self.cursor_col] = self.current_char();
    }

    /// Erase the cell at cursor position.
    pub fn erase(&mut self) {
        self.grid[self.cursor_row][self.cursor_col] = ' ';
    }
}
