//! Built-in chess engine with adjustable strength and personalities.
//! Uses minimax with alpha-beta pruning and piece-square tables.

use crate::board::*;

// ── Piece Values ───────────────────────────────────────────────────

const PAWN_VAL: i32 = 100;
const KNIGHT_VAL: i32 = 320;
const BISHOP_VAL: i32 = 330;
const ROOK_VAL: i32 = 500;
const QUEEN_VAL: i32 = 900;
const KING_VAL: i32 = 20000;

fn piece_value(pt: usize) -> i32 {
    match pt {
        PAWN => PAWN_VAL,
        KNIGHT => KNIGHT_VAL,
        BISHOP => BISHOP_VAL,
        ROOK => ROOK_VAL,
        QUEEN => QUEEN_VAL,
        KING => KING_VAL,
        _ => 0,
    }
}

// ── Piece-Square Tables (from White's perspective) ─────────────────
// Bonus for having a piece on a given square. Encourages good placement.

#[rustfmt::skip]
const PAWN_TABLE: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10,-20,-20, 10, 10,  5,
     5, -5,-10,  0,  0,-10, -5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5,  5, 10, 25, 25, 10,  5,  5,
    10, 10, 20, 30, 30, 20, 10, 10,
    50, 50, 50, 50, 50, 50, 50, 50,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const KNIGHT_TABLE: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
const BISHOP_TABLE: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const ROOK_TABLE: [i32; 64] = [
     0,  0,  0,  5,  5,  0,  0,  0,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     5, 10, 10, 10, 10, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const QUEEN_TABLE: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -10,  5,  5,  5,  5,  5,  0,-10,
      0,  0,  5,  5,  5,  5,  0, -5,
     -5,  0,  5,  5,  5,  5,  0, -5,
    -10,  0,  5,  5,  5,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];

#[rustfmt::skip]
const KING_TABLE_MID: [i32; 64] = [
     20, 30, 10,  0,  0, 10, 30, 20,
     20, 20,  0,  0,  0,  0, 20, 20,
    -10,-20,-20,-20,-20,-20,-20,-10,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
];

fn pst_value(pt: usize, sq: u8, is_white: bool) -> i32 {
    // Mirror for black pieces
    let idx = if is_white { sq as usize } else { (56 - (sq & !7) + (sq & 7)) as usize };
    match pt {
        PAWN => PAWN_TABLE[idx],
        KNIGHT => KNIGHT_TABLE[idx],
        BISHOP => BISHOP_TABLE[idx],
        ROOK => ROOK_TABLE[idx],
        QUEEN => QUEEN_TABLE[idx],
        KING => KING_TABLE_MID[idx],
        _ => 0,
    }
}

// ── Evaluation ─────────────────────────────────────────────────────

fn evaluate(pos: &Position, personality: &Personality) -> i32 {
    let mut score: i32 = 0;

    for pt in 0..6 {
        // White pieces
        let mut white = pos.pieces[pt] & pos.colors[WHITE];
        while white != 0 {
            let sq = white.trailing_zeros() as u8;
            score += piece_value(pt) + pst_value(pt, sq, true);
            white &= white - 1;
        }
        // Black pieces
        let mut black = pos.pieces[pt] & pos.colors[BLACK];
        while black != 0 {
            let sq = black.trailing_zeros() as u8;
            score -= piece_value(pt) + pst_value(pt, sq, false);
            black &= black - 1;
        }
    }

    // Personality adjustments
    let mobility_white = pos.legal_moves().len() as i32;
    // Approximate black mobility
    let mut flipped = pos.clone();
    flipped.side_to_move = if pos.side_to_move == Color::White { Color::Black } else { Color::White };
    let mobility_diff = mobility_white as i32 * personality.aggression / 10;
    score += mobility_diff;

    // Return from the perspective of the side to move
    if pos.side_to_move == Color::White { score } else { -score }
}

// ── Search ─────────────────────────────────────────────────────────

fn alpha_beta(pos: &Position, depth: u8, mut alpha: i32, beta: i32, personality: &Personality) -> i32 {
    if depth == 0 {
        return evaluate(pos, personality);
    }

    let moves = pos.legal_moves();
    if moves.is_empty() {
        if pos.in_check(pos.side_to_move) {
            return -KING_VAL + (10 - depth as i32); // Prefer faster mates
        }
        return 0; // Stalemate
    }

    for mv in moves {
        let new_pos = pos.make_move(mv);
        let score = -alpha_beta(&new_pos, depth - 1, -beta, -alpha, personality);
        if score >= beta {
            return beta;
        }
        if score > alpha {
            alpha = score;
        }
    }
    alpha
}

// ── Personality ────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Personality {
    pub name: &'static str,
    pub description: &'static str,
    pub depth: u8,          // search depth (1-7)
    pub aggression: i32,    // 0-20, how much mobility matters
    pub randomness: u8,     // 0-100, chance of picking a suboptimal move
}

pub const PERSONALITIES: &[Personality] = &[
    Personality {
        name: "Beginner",
        description: "Just learning the rules. Makes random mistakes.",
        depth: 1, aggression: 5, randomness: 40,
    },
    Personality {
        name: "Casual",
        description: "Plays for fun. Occasionally blunders.",
        depth: 2, aggression: 8, randomness: 20,
    },
    Personality {
        name: "Club Player",
        description: "Solid fundamentals. Doesn't hang pieces.",
        depth: 3, aggression: 10, randomness: 5,
    },
    Personality {
        name: "Strong Player",
        description: "Plays well. Finds tactical shots.",
        depth: 4, aggression: 12, randomness: 2,
    },
    Personality {
        name: "Expert",
        description: "Tournament strength. Deep calculation.",
        depth: 5, aggression: 15, randomness: 0,
    },
    // Famous player styles
    Personality {
        name: "Bobby Fischer",
        description: "Aggressive, precise, no mercy.",
        depth: 4, aggression: 18, randomness: 0,
    },
    Personality {
        name: "Mikhail Tal",
        description: "Wild sacrifices, chaotic attacks.",
        depth: 3, aggression: 20, randomness: 10,
    },
    Personality {
        name: "Anatoly Karpov",
        description: "Positional squeeze, quiet suffocation.",
        depth: 4, aggression: 5, randomness: 0,
    },
    Personality {
        name: "Garry Kasparov",
        description: "Dynamic, calculating, overwhelming force.",
        depth: 5, aggression: 16, randomness: 0,
    },
    Personality {
        name: "Magnus Carlsen",
        description: "Universal, patient, grinds you down.",
        depth: 5, aggression: 12, randomness: 0,
    },
];

// ── Engine API ─────────────────────────────────────────────────────

/// Find the best move for the current position with the given personality.
/// Runs on the calling thread — call from a background thread for non-blocking.
pub fn find_best_move(pos: &Position, personality: &Personality) -> Option<Move> {
    let moves = pos.legal_moves();
    if moves.is_empty() {
        return None;
    }

    // Score all moves
    let mut scored: Vec<(Move, i32)> = moves.iter().map(|&mv| {
        let new_pos = pos.make_move(mv);
        let score = -alpha_beta(&new_pos, personality.depth.saturating_sub(1), -KING_VAL, KING_VAL, personality);
        (mv, score)
    }).collect();

    // Sort by score (best first)
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    // Apply randomness — sometimes pick a suboptimal move
    if personality.randomness > 0 && scored.len() > 1 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
        let mut hasher = DefaultHasher::new();
        nanos.hash(&mut hasher);
        let rand = (hasher.finish() % 100) as u8;

        if rand < personality.randomness {
            // Pick a random move from the top half
            let range = (scored.len() / 2).max(2).min(scored.len());
            let idx = (hasher.finish() / 100 % range as u64) as usize;
            return Some(scored[idx].0);
        }
    }

    Some(scored[0].0)
}
