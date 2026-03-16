#![allow(dead_code)]

pub const PAWN: usize = 0;
pub const KNIGHT: usize = 1;
pub const BISHOP: usize = 2;
pub const ROOK: usize = 3;
pub const QUEEN: usize = 4;
pub const KING: usize = 5;

pub const WHITE: usize = 0;
pub const BLACK: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

#[derive(Clone)]
pub struct Position {
    pub pieces: [u64; 6],
    pub colors: [u64; 2],
    pub side_to_move: Color,
    pub castling: u8,
    pub en_passant: Option<u8>,
}

impl Position {
    pub fn empty() -> Self {
        Self {
            pieces: [0; 6],
            colors: [0; 2],
            side_to_move: Color::White,
            castling: 0,
            en_passant: None,
        }
    }

    pub fn start() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn from_fen(fen: &str) -> Option<Self> {
        let mut pos = Self::empty();
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        // Parse piece placement
        let mut rank: i8 = 7;
        let mut file: i8 = 0;

        for ch in parts[0].chars() {
            match ch {
                '/' => {
                    rank -= 1;
                    file = 0;
                }
                '1'..='8' => {
                    file += ch as i8 - '0' as i8;
                }
                _ => {
                    if rank < 0 || file > 7 {
                        return None;
                    }
                    let sq = (rank * 8 + file) as u8;
                    let bit = 1u64 << sq;

                    let (piece_type, color) = match ch {
                        'P' => (PAWN, WHITE),
                        'N' => (KNIGHT, WHITE),
                        'B' => (BISHOP, WHITE),
                        'R' => (ROOK, WHITE),
                        'Q' => (QUEEN, WHITE),
                        'K' => (KING, WHITE),
                        'p' => (PAWN, BLACK),
                        'n' => (KNIGHT, BLACK),
                        'b' => (BISHOP, BLACK),
                        'r' => (ROOK, BLACK),
                        'q' => (QUEEN, BLACK),
                        'k' => (KING, BLACK),
                        _ => return None,
                    };

                    pos.pieces[piece_type] |= bit;
                    pos.colors[color] |= bit;
                    file += 1;
                }
            }
        }

        // Side to move
        if parts.len() > 1 {
            pos.side_to_move = match parts[1] {
                "w" => Color::White,
                "b" => Color::Black,
                _ => return None,
            };
        }

        // Castling
        if parts.len() > 2 {
            for ch in parts[2].chars() {
                match ch {
                    'K' => pos.castling |= 0b0001,
                    'Q' => pos.castling |= 0b0010,
                    'k' => pos.castling |= 0b0100,
                    'q' => pos.castling |= 0b1000,
                    '-' => {}
                    _ => {}
                }
            }
        }

        // En passant
        if parts.len() > 3 && parts[3] != "-" {
            let bytes = parts[3].as_bytes();
            if bytes.len() == 2 {
                let ep_file = bytes[0] - b'a';
                let ep_rank = bytes[1] - b'1';
                pos.en_passant = Some(ep_rank * 8 + ep_file);
            }
        }

        Some(pos)
    }

    /// Get the piece type at a square (0-63), or None if empty.
    pub fn piece_at(&self, sq: u8) -> Option<(usize, Color)> {
        let bit = 1u64 << sq;
        let occupied = self.colors[WHITE] | self.colors[BLACK];
        if occupied & bit == 0 {
            return None;
        }

        let color = if self.colors[WHITE] & bit != 0 {
            Color::White
        } else {
            Color::Black
        };

        for piece_type in 0..6 {
            if self.pieces[piece_type] & bit != 0 {
                return Some((piece_type, color));
            }
        }

        None
    }
}

pub fn sq_to_file_rank(sq: u8) -> (u8, u8) {
    (sq % 8, sq / 8)
}

pub fn file_rank_to_sq(file: u8, rank: u8) -> u8 {
    rank * 8 + file
}

// ── Moves ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub promotion: Option<usize>, // piece type to promote to
}

impl Move {
    pub fn new(from: u8, to: u8) -> Self {
        Self { from, to, promotion: None }
    }

    pub fn with_promotion(from: u8, to: u8, promo: usize) -> Self {
        Self { from, to, promotion: Some(promo) }
    }

    /// Parse UCI format like "e2e4" or "a7a8q"
    pub fn from_uci(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() < 4 { return None; }
        let from_file = bytes[0].wrapping_sub(b'a');
        let from_rank = bytes[1].wrapping_sub(b'1');
        let to_file = bytes[2].wrapping_sub(b'a');
        let to_rank = bytes[3].wrapping_sub(b'1');
        if from_file > 7 || from_rank > 7 || to_file > 7 || to_rank > 7 {
            return None;
        }
        let from = from_rank * 8 + from_file;
        let to = to_rank * 8 + to_file;
        let promotion = if bytes.len() > 4 {
            match bytes[4] {
                b'q' => Some(QUEEN),
                b'r' => Some(ROOK),
                b'b' => Some(BISHOP),
                b'n' => Some(KNIGHT),
                _ => None,
            }
        } else {
            None
        };
        Some(Self { from, to, promotion })
    }

    pub fn to_uci(&self) -> String {
        let ff = (self.from % 8 + b'a') as char;
        let fr = (self.from / 8 + b'1') as char;
        let tf = (self.to % 8 + b'a') as char;
        let tr = (self.to / 8 + b'1') as char;
        let promo = match self.promotion {
            Some(QUEEN) => "q",
            Some(ROOK) => "r",
            Some(BISHOP) => "b",
            Some(KNIGHT) => "n",
            _ => "",
        };
        format!("{ff}{fr}{tf}{tr}{promo}")
    }
}

// ── Attack Tables ──────────────────────────────────────────────────

/// Knight attack bitboard for a given square.
pub fn knight_attacks(sq: u8) -> u64 {
    let mut attacks = 0u64;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    for &(dr, df) in &[(-2,-1),(-2,1),(-1,-2),(-1,2),(1,-2),(1,2),(2,-1),(2,1)] {
        let nr = r + dr;
        let nf = f + df;
        if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }
    attacks
}

/// King attack bitboard for a given square.
pub fn king_attacks(sq: u8) -> u64 {
    let mut attacks = 0u64;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    for &(dr, df) in &[(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)] {
        let nr = r + dr;
        let nf = f + df;
        if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }
    attacks
}

/// Sliding piece ray in one direction, stopping at blockers.
fn ray(sq: u8, dr: i8, df: i8, occupied: u64) -> u64 {
    let mut attacks = 0u64;
    let mut r = (sq / 8) as i8 + dr;
    let mut f = (sq % 8) as i8 + df;
    while r >= 0 && r < 8 && f >= 0 && f < 8 {
        let bit = 1u64 << (r * 8 + f);
        attacks |= bit;
        if occupied & bit != 0 { break; } // hit a piece, stop
        r += dr;
        f += df;
    }
    attacks
}

/// Bishop attacks (diagonals).
pub fn bishop_attacks(sq: u8, occupied: u64) -> u64 {
    ray(sq, -1, -1, occupied) | ray(sq, -1, 1, occupied) |
    ray(sq, 1, -1, occupied) | ray(sq, 1, 1, occupied)
}

/// Rook attacks (ranks and files).
pub fn rook_attacks(sq: u8, occupied: u64) -> u64 {
    ray(sq, -1, 0, occupied) | ray(sq, 1, 0, occupied) |
    ray(sq, 0, -1, occupied) | ray(sq, 0, 1, occupied)
}

/// Queen attacks (rook + bishop).
pub fn queen_attacks(sq: u8, occupied: u64) -> u64 {
    rook_attacks(sq, occupied) | bishop_attacks(sq, occupied)
}

/// Pawn attacks (captures only, not pushes).
pub fn pawn_attacks(sq: u8, color: Color) -> u64 {
    let mut attacks = 0u64;
    let f = (sq % 8) as i8;
    let r = (sq / 8) as i8;
    let dir: i8 = if color == Color::White { 1 } else { -1 };
    let nr = r + dir;
    if nr >= 0 && nr < 8 {
        if f > 0 { attacks |= 1u64 << (nr * 8 + f - 1); }
        if f < 7 { attacks |= 1u64 << (nr * 8 + f + 1); }
    }
    attacks
}

/// All squares attacked by a given side.
pub fn attacked_by(pos: &Position, color: Color) -> u64 {
    let us = if color == Color::White { WHITE } else { BLACK };
    let occupied = pos.colors[WHITE] | pos.colors[BLACK];
    let mut attacks = 0u64;

    // Pawns
    let mut pawns = pos.pieces[PAWN] & pos.colors[us];
    while pawns != 0 {
        let sq = pawns.trailing_zeros() as u8;
        attacks |= pawn_attacks(sq, color);
        pawns &= pawns - 1;
    }
    // Knights
    let mut knights = pos.pieces[KNIGHT] & pos.colors[us];
    while knights != 0 {
        let sq = knights.trailing_zeros() as u8;
        attacks |= knight_attacks(sq);
        knights &= knights - 1;
    }
    // Bishops
    let mut bishops = pos.pieces[BISHOP] & pos.colors[us];
    while bishops != 0 {
        let sq = bishops.trailing_zeros() as u8;
        attacks |= bishop_attacks(sq, occupied);
        bishops &= bishops - 1;
    }
    // Rooks
    let mut rooks = pos.pieces[ROOK] & pos.colors[us];
    while rooks != 0 {
        let sq = rooks.trailing_zeros() as u8;
        attacks |= rook_attacks(sq, occupied);
        rooks &= rooks - 1;
    }
    // Queens
    let mut queens = pos.pieces[QUEEN] & pos.colors[us];
    while queens != 0 {
        let sq = queens.trailing_zeros() as u8;
        attacks |= queen_attacks(sq, occupied);
        queens &= queens - 1;
    }
    // King
    let king_sq = (pos.pieces[KING] & pos.colors[us]).trailing_zeros() as u8;
    if king_sq < 64 {
        attacks |= king_attacks(king_sq);
    }

    attacks
}

// ── Move Generation ────────────────────────────────────────────────

impl Position {
    fn color_index(&self) -> usize {
        if self.side_to_move == Color::White { WHITE } else { BLACK }
    }

    fn enemy_index(&self) -> usize {
        if self.side_to_move == Color::White { BLACK } else { WHITE }
    }

    fn occupied(&self) -> u64 {
        self.colors[WHITE] | self.colors[BLACK]
    }

    /// Is the given side's king in check?
    pub fn in_check(&self, color: Color) -> bool {
        let us = if color == Color::White { WHITE } else { BLACK };
        let king_sq = (self.pieces[KING] & self.colors[us]).trailing_zeros() as u8;
        if king_sq >= 64 { return false; }
        let enemy = if color == Color::White { Color::Black } else { Color::White };
        attacked_by(self, enemy) & (1u64 << king_sq) != 0
    }

    /// Apply a move and return the new position. Does not check legality.
    pub fn make_move(&self, mv: Move) -> Self {
        let mut pos = self.clone();
        let us = pos.color_index();
        let them = pos.enemy_index();
        let from_bit = 1u64 << mv.from;
        let to_bit = 1u64 << mv.to;

        // Find the piece being moved
        let mut piece_type = PAWN; // default
        for pt in 0..6 {
            if pos.pieces[pt] & from_bit != 0 {
                piece_type = pt;
                break;
            }
        }

        // Remove captured piece (if any)
        if pos.colors[them] & to_bit != 0 {
            for pt in 0..6 {
                pos.pieces[pt] &= !to_bit;
            }
            pos.colors[them] &= !to_bit;
        }

        // Move the piece
        pos.pieces[piece_type] &= !from_bit;
        pos.colors[us] &= !from_bit;

        if let Some(promo) = mv.promotion {
            pos.pieces[promo] |= to_bit;
        } else {
            pos.pieces[piece_type] |= to_bit;
        }
        pos.colors[us] |= to_bit;

        // En passant capture
        if piece_type == PAWN && Some(mv.to) == self.en_passant {
            let captured_sq = if self.side_to_move == Color::White {
                mv.to - 8
            } else {
                mv.to + 8
            };
            let cap_bit = 1u64 << captured_sq;
            pos.pieces[PAWN] &= !cap_bit;
            pos.colors[them] &= !cap_bit;
        }

        // Update en passant square
        pos.en_passant = None;
        if piece_type == PAWN {
            let diff = (mv.to as i8 - mv.from as i8).unsigned_abs();
            if diff == 16 {
                pos.en_passant = Some((mv.from + mv.to) / 2);
            }
        }

        // Castling — move the rook
        if piece_type == KING {
            let diff = mv.to as i8 - mv.from as i8;
            if diff == 2 {
                // Kingside
                let rook_from = mv.from + 3;
                let rook_to = mv.from + 1;
                pos.pieces[ROOK] &= !(1u64 << rook_from);
                pos.colors[us] &= !(1u64 << rook_from);
                pos.pieces[ROOK] |= 1u64 << rook_to;
                pos.colors[us] |= 1u64 << rook_to;
            } else if diff == -2 {
                // Queenside
                let rook_from = mv.from - 4;
                let rook_to = mv.from - 1;
                pos.pieces[ROOK] &= !(1u64 << rook_from);
                pos.colors[us] &= !(1u64 << rook_from);
                pos.pieces[ROOK] |= 1u64 << rook_to;
                pos.colors[us] |= 1u64 << rook_to;
            }
        }

        // Update castling rights
        if piece_type == KING {
            if self.side_to_move == Color::White {
                pos.castling &= !0b0011;
            } else {
                pos.castling &= !0b1100;
            }
        }
        // Rook moved or captured — revoke that side's castling
        for &(sq, mask) in &[(0u8, 0b0010u8), (7, 0b0001), (56, 0b1000), (63, 0b0100)] {
            if mv.from == sq || mv.to == sq {
                pos.castling &= !mask;
            }
        }

        pos.side_to_move = if self.side_to_move == Color::White {
            Color::Black
        } else {
            Color::White
        };

        pos
    }

    /// Generate all legal moves for the side to move.
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        self.generate_pseudo_legal(&mut moves);
        // Filter to legal: king must not be in check after the move
        let us = self.side_to_move;
        moves.retain(|mv| {
            let new_pos = self.make_move(*mv);
            !new_pos.in_check(us)
        });
        moves
    }

    fn generate_pseudo_legal(&self, moves: &mut Vec<Move>) {
        let us = self.color_index();
        let them = self.enemy_index();
        let friendly = self.colors[us];
        let occupied = self.occupied();

        // Pawns
        self.gen_pawn_moves(moves, us, them, friendly, occupied);
        // Knights
        self.gen_piece_moves(moves, KNIGHT, friendly, |sq, _occ| knight_attacks(sq), occupied);
        // Bishops
        self.gen_piece_moves(moves, BISHOP, friendly, bishop_attacks, occupied);
        // Rooks
        self.gen_piece_moves(moves, ROOK, friendly, rook_attacks, occupied);
        // Queens
        self.gen_piece_moves(moves, QUEEN, friendly, queen_attacks, occupied);
        // King
        self.gen_piece_moves(moves, KING, friendly, |sq, _occ| king_attacks(sq), occupied);
        // Castling
        self.gen_castling(moves, us, occupied);
    }

    fn gen_piece_moves(
        &self,
        moves: &mut Vec<Move>,
        piece_type: usize,
        friendly: u64,
        attack_fn: fn(u8, u64) -> u64,
        occupied: u64,
    ) {
        let mut bb = self.pieces[piece_type] & self.colors[self.color_index()];
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let targets = attack_fn(sq, occupied) & !friendly;
            let mut t = targets;
            while t != 0 {
                let to = t.trailing_zeros() as u8;
                moves.push(Move::new(sq, to));
                t &= t - 1;
            }
            bb &= bb - 1;
        }
    }

    fn gen_pawn_moves(&self, moves: &mut Vec<Move>, us: usize, them: usize, _friendly: u64, occupied: u64) {
        let pawns = self.pieces[PAWN] & self.colors[us];
        let enemy = self.colors[them];
        let empty = !occupied;
        let (dir, start_rank, promo_rank): (i8, u64, u64) = if us == WHITE {
            (8, 0x000000000000FF00, 0x00FF000000000000)
        } else {
            (-8, 0x00FF000000000000, 0x000000000000FF00)
        };

        let mut bb = pawns;
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let from_bit = 1u64 << sq;
            let to_sq = (sq as i8 + dir) as u8;

            // Single push
            if to_sq < 64 && empty & (1u64 << to_sq) != 0 {
                if from_bit & promo_rank != 0 {
                    // Pawn on 7th/2nd — next push is promotion
                    // Actually check if TO square is on promo rank
                } else if (1u64 << to_sq) & (if us == WHITE { 0xFF00000000000000 } else { 0x00000000000000FF }) != 0 {
                    for &promo in &[QUEEN, ROOK, BISHOP, KNIGHT] {
                        moves.push(Move::with_promotion(sq, to_sq, promo));
                    }
                } else {
                    moves.push(Move::new(sq, to_sq));
                }

                // Double push from start rank
                if from_bit & start_rank != 0 {
                    let to_sq2 = (sq as i8 + dir * 2) as u8;
                    if to_sq2 < 64 && empty & (1u64 << to_sq2) != 0 {
                        moves.push(Move::new(sq, to_sq2));
                    }
                }
            }

            // Captures
            let attacks = pawn_attacks(sq, self.side_to_move);
            let mut caps = attacks & enemy;
            while caps != 0 {
                let to = caps.trailing_zeros() as u8;
                let to_bit = 1u64 << to;
                if to_bit & (if us == WHITE { 0xFF00000000000000 } else { 0x00000000000000FF }) != 0 {
                    for &promo in &[QUEEN, ROOK, BISHOP, KNIGHT] {
                        moves.push(Move::with_promotion(sq, to, promo));
                    }
                } else {
                    moves.push(Move::new(sq, to));
                }
                caps &= caps - 1;
            }

            // En passant
            if let Some(ep) = self.en_passant {
                if attacks & (1u64 << ep) != 0 {
                    moves.push(Move::new(sq, ep));
                }
            }

            bb &= bb - 1;
        }
    }

    fn gen_castling(&self, moves: &mut Vec<Move>, us: usize, occupied: u64) {
        let enemy_color = if us == WHITE { Color::Black } else { Color::White };
        let enemy_attacks = attacked_by(self, enemy_color);

        if us == WHITE {
            let king_sq = 4u8;
            // Kingside
            if self.castling & 0b0001 != 0
                && occupied & 0x60 == 0 // f1, g1 empty
                && enemy_attacks & 0x70 == 0 // e1, f1, g1 not attacked
            {
                moves.push(Move::new(king_sq, 6));
            }
            // Queenside
            if self.castling & 0b0010 != 0
                && occupied & 0x0E == 0 // b1, c1, d1 empty
                && enemy_attacks & 0x1C == 0 // c1, d1, e1 not attacked
            {
                moves.push(Move::new(king_sq, 2));
            }
        } else {
            let king_sq = 60u8;
            // Kingside
            if self.castling & 0b0100 != 0
                && occupied & (0x60u64 << 56) == 0
                && enemy_attacks & (0x70u64 << 56) == 0
            {
                moves.push(Move::new(king_sq, 62));
            }
            // Queenside
            if self.castling & 0b1000 != 0
                && occupied & (0x0Eu64 << 56) == 0
                && enemy_attacks & (0x1Cu64 << 56) == 0
            {
                moves.push(Move::new(king_sq, 58));
            }
        }
    }

    /// Is the current position checkmate?
    pub fn is_checkmate(&self) -> bool {
        self.in_check(self.side_to_move) && self.legal_moves().is_empty()
    }

    /// Is the current position stalemate?
    pub fn is_stalemate(&self) -> bool {
        !self.in_check(self.side_to_move) && self.legal_moves().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_position() {
        let pos = Position::start();
        assert!(pos.pieces[KING] & pos.colors[WHITE] & (1u64 << 4) != 0);
        assert!(pos.pieces[KING] & pos.colors[BLACK] & (1u64 << 60) != 0);
        assert_eq!(pos.colors[WHITE].count_ones(), 16);
        assert_eq!(pos.colors[BLACK].count_ones(), 16);
    }

    #[test]
    fn test_piece_at() {
        let pos = Position::start();
        assert_eq!(pos.piece_at(4), Some((KING, Color::White)));
        assert_eq!(pos.piece_at(60), Some((KING, Color::Black)));
        assert_eq!(pos.piece_at(28), None);
    }

    #[test]
    fn test_start_legal_moves() {
        let pos = Position::start();
        let moves = pos.legal_moves();
        // 20 legal moves in starting position: 16 pawn + 4 knight
        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn test_uci_parse() {
        let mv = Move::from_uci("e2e4").unwrap();
        assert_eq!(mv.from, 12); // e2
        assert_eq!(mv.to, 28);   // e4
        assert_eq!(mv.promotion, None);

        let mv = Move::from_uci("a7a8q").unwrap();
        assert_eq!(mv.promotion, Some(QUEEN));
    }

    #[test]
    fn test_uci_roundtrip() {
        let mv = Move::new(12, 28);
        assert_eq!(mv.to_uci(), "e2e4");
        let mv = Move::with_promotion(52, 60, QUEEN);
        assert_eq!(mv.to_uci(), "e7e8q");
    }

    #[test]
    fn test_make_move() {
        let pos = Position::start();
        let mv = Move::from_uci("e2e4").unwrap();
        let new_pos = pos.make_move(mv);
        // Pawn on e4 now
        assert!(new_pos.pieces[PAWN] & (1u64 << 28) != 0);
        // No pawn on e2
        assert!(new_pos.pieces[PAWN] & (1u64 << 12) == 0);
        // Black to move
        assert_eq!(new_pos.side_to_move, Color::Black);
        // En passant on e3
        assert_eq!(new_pos.en_passant, Some(20));
    }

    #[test]
    fn test_in_check() {
        // Scholar's mate position — black king in check
        let pos = Position::from_fen("r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4").unwrap();
        assert!(pos.in_check(Color::Black));
        assert!(pos.is_checkmate());
    }

    #[test]
    fn test_not_checkmate() {
        let pos = Position::start();
        assert!(!pos.is_checkmate());
        assert!(!pos.is_stalemate());
    }

    #[test]
    fn test_castling() {
        // Position where white can castle kingside
        let pos = Position::from_fen("r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4").unwrap();
        let moves = pos.legal_moves();
        let castle = moves.iter().find(|m| m.from == 4 && m.to == 6);
        assert!(castle.is_some(), "Kingside castling should be legal");
    }

    #[test]
    fn test_en_passant() {
        // White pawn on e5, black just played d7d5
        let pos = Position::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
        let moves = pos.legal_moves();
        let ep = moves.iter().find(|m| m.from == 36 && m.to == 43); // e5 to d6
        assert!(ep.is_some(), "En passant should be legal");
    }

    #[test]
    fn test_knight_attacks() {
        // Knight on e4 (sq 28) should attack 8 squares
        let attacks = knight_attacks(28);
        assert_eq!(attacks.count_ones(), 8);
        // Knight on a1 (sq 0) should attack 2 squares
        let attacks = knight_attacks(0);
        assert_eq!(attacks.count_ones(), 2);
    }

    #[test]
    fn test_stalemate() {
        // King vs King + Queen, black king in corner, stalemate
        let pos = Position::from_fen("k7/2Q5/1K6/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(pos.is_stalemate());
    }
}
