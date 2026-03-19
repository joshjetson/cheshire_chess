//! Mini-games for chess skill training.
//! Knight's Tour, Blindfold Mode, Color Quiz, Pawn Race.

use crate::board::Position;

// ── Knight's Tour ──────────────────────────────────────────────────

/// The player must move a knight to visit every square exactly once.
pub struct KnightTour {
    pub knight_sq: u8,
    pub visited: [bool; 64],
    pub visit_count: usize,
    pub move_history: Vec<u8>,
}

impl KnightTour {
    pub fn new(start: u8) -> Self {
        let mut visited = [false; 64];
        visited[start as usize] = true;
        Self {
            knight_sq: start,
            visited,
            visit_count: 1,
            move_history: vec![start],
        }
    }

    /// Get all legal knight moves from current square that haven't been visited.
    pub fn legal_moves(&self) -> Vec<u8> {
        let r = (self.knight_sq / 8) as i8;
        let f = (self.knight_sq % 8) as i8;
        let mut moves = Vec::new();
        for &(dr, df) in &[(-2,-1),(-2,1),(-1,-2),(-1,2),(1,-2),(1,2),(2,-1),(2,1)] {
            let nr = r + dr;
            let nf = f + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                let sq = (nr * 8 + nf) as u8;
                if !self.visited[sq as usize] {
                    moves.push(sq);
                }
            }
        }
        moves
    }

    /// Try to move to a square. Returns true if successful.
    pub fn try_move(&mut self, to: u8) -> bool {
        if self.legal_moves().contains(&to) {
            self.knight_sq = to;
            self.visited[to as usize] = true;
            self.visit_count += 1;
            self.move_history.push(to);
            true
        } else {
            false
        }
    }

    /// Undo the last move.
    pub fn undo(&mut self) {
        if self.move_history.len() > 1 {
            let last = self.move_history.pop().unwrap();
            self.visited[last as usize] = false;
            self.visit_count -= 1;
            self.knight_sq = *self.move_history.last().unwrap();
        }
    }

    pub fn is_complete(&self) -> bool {
        self.visit_count == 64
    }

    pub fn is_stuck(&self) -> bool {
        self.legal_moves().is_empty() && !self.is_complete()
    }

    /// Build a Position with just the knight for display.
    pub fn to_position(&self) -> Position {
        let mut pos = Position::empty();
        pos.pieces[crate::board::KNIGHT] = 1u64 << self.knight_sq;
        pos.colors[crate::board::WHITE] = 1u64 << self.knight_sq;
        pos
    }

    /// Get a u64 mask of visited squares (for highlighting).
    pub fn visited_mask(&self) -> Vec<u8> {
        self.visited.iter().enumerate()
            .filter(|(i, &v)| v && *i as u8 != self.knight_sq)
            .map(|(i, _)| i as u8)
            .collect()
    }
}

// ── Color Quiz ─────────────────────────────────────────────────────

/// Flash a square name, player answers if it's light or dark.
pub struct ColorQuiz {
    pub current_square: u8,
    pub score: usize,
    pub total: usize,
    pub streak: usize,
    pub best_streak: usize,
}

impl ColorQuiz {
    pub fn new() -> Self {
        Self {
            current_square: Self::random_square(),
            score: 0,
            total: 0,
            streak: 0,
            best_streak: 0,
        }
    }

    fn random_square() -> u8 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
        let mut hasher = DefaultHasher::new();
        nanos.hash(&mut hasher);
        (hasher.finish() % 64) as u8
    }

    pub fn square_name(&self) -> String {
        let file = (b'a' + self.current_square % 8) as char;
        let rank = self.current_square / 8 + 1;
        format!("{file}{rank}")
    }

    pub fn is_light(&self) -> bool {
        let rank = self.current_square / 8;
        let file = self.current_square % 8;
        (rank + file) % 2 == 1
    }

    /// Player guesses. Returns true if correct.
    pub fn guess(&mut self, light: bool) -> bool {
        self.total += 1;
        let correct = light == self.is_light();
        if correct {
            self.score += 1;
            self.streak += 1;
            if self.streak > self.best_streak {
                self.best_streak = self.streak;
            }
        } else {
            self.streak = 0;
        }
        self.current_square = Self::random_square();
        correct
    }
}

// ── Blindfold Mode ─────────────────────────────────────────────────

/// Standard game but pieces are hidden. Toggle visibility.
#[allow(dead_code)]
pub struct BlindFold {
    pub hidden: bool,
    pub position: Position,
    pub peek_count: usize,
}

#[allow(dead_code)]
impl BlindFold {
    pub fn new() -> Self {
        Self {
            hidden: true,
            position: Position::start(),
            peek_count: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.hidden = !self.hidden;
        if !self.hidden {
            self.peek_count += 1;
        }
    }
}

// ── Mini-game menu ─────────────────────────────────────────────────

pub const MINIGAME_LIST: &[(&str, &str)] = &[
    ("Knight's Tour", "Visit all 64 squares with a knight"),
    ("Color Quiz", "Is the square light or dark? Test your speed"),
    ("Blindfold Chess", "Play without seeing the pieces"),
];