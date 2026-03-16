use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::fs::File;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A chess puzzle loaded on demand.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Puzzle {
    pub id: String,
    pub fen: String,
    pub moves: Vec<String>,
    pub rating: u16,
    pub themes: Vec<String>,
}

impl Puzzle {
    pub fn solution_moves(&self) -> &[String] {
        if self.moves.len() > 1 {
            &self.moves[1..]
        } else {
            &[]
        }
    }

    fn from_csv_line(line: &str) -> Option<Self> {
        let mut fields = CsvFieldIter::new(line);
        let id = fields.next()?;
        let fen = fields.next()?;
        let moves_str = fields.next()?;
        let rating_str = fields.next()?;
        // skip RatingDeviation, Popularity, NbPlays
        fields.next()?;
        fields.next()?;
        fields.next()?;
        let themes_str = fields.next()?;

        let rating: u16 = rating_str.parse().ok()?;
        let moves: Vec<String> = moves_str.split_whitespace().map(String::from).collect();
        let themes: Vec<String> = themes_str.split_whitespace().map(String::from).collect();

        Some(Puzzle {
            id: id.to_string(),
            fen: fen.to_string(),
            moves,
            rating,
            themes,
        })
    }
}

/// Simple CSV field iterator that handles the common case (no quoted fields with commas).
struct CsvFieldIter<'a> {
    remaining: &'a str,
}

impl<'a> CsvFieldIter<'a> {
    fn new(line: &'a str) -> Self {
        Self { remaining: line }
    }

    fn next(&mut self) -> Option<&'a str> {
        if self.remaining.is_empty() {
            return None;
        }
        match self.remaining.find(',') {
            Some(pos) => {
                let field = &self.remaining[..pos];
                self.remaining = &self.remaining[pos + 1..];
                Some(field)
            }
            None => {
                let field = self.remaining;
                self.remaining = "";
                Some(field)
            }
        }
    }
}

/// Pre-built index: theme -> list of byte offsets into the CSV file.
/// Also stores per-theme counts so the UI never has to scan.
pub struct PuzzleIndex {
    path: PathBuf,
    theme_offsets: HashMap<String, Vec<u64>>,
    pub theme_counts: Vec<(String, String, usize)>, // (tag, display_name, count)
    pub total: usize,
}

impl PuzzleIndex {
    /// Single-pass index build. Reads the file line by line, records byte
    /// offsets per theme. No puzzle data is kept in memory.
    pub fn build(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(256 * 1024, file);
        let mut theme_offsets: HashMap<String, Vec<u64>> = HashMap::new();
        let mut line = String::new();
        let mut offset: u64 = 0;
        let mut total: usize = 0;

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim_end();
            // Quick parse: find the 8th field (themes) without allocating
            if let Some(themes_field) = nth_csv_field(trimmed, 7) {
                for theme in themes_field.split_whitespace() {
                    theme_offsets
                        .entry(theme.to_string())
                        .or_default()
                        .push(offset);
                }
            }

            offset += bytes_read as u64;
            total += 1;
        }

        // Build counts for display themes
        let theme_counts: Vec<(String, String, usize)> = TACTIC_THEMES
            .iter()
            .map(|&(tag, name)| {
                let count = theme_offsets.get(tag).map_or(0, |v| v.len());
                (tag.to_string(), name.to_string(), count)
            })
            .collect();

        Ok(Self {
            path: path.to_path_buf(),
            theme_offsets,
            theme_counts,
            total,
        })
    }

    /// Load puzzles for a theme, optionally capped and filtered by rating.
    pub fn load_theme(
        &self,
        theme: &str,
        max_rating: Option<u16>,
        limit: usize,
    ) -> io::Result<Vec<Puzzle>> {
        let offsets = match self.theme_offsets.get(theme) {
            Some(o) => o,
            None => return Ok(Vec::new()),
        };

        let mut file = File::open(&self.path)?;
        let mut puzzles = Vec::with_capacity(limit.min(offsets.len()));
        let mut line_buf = String::new();

        for &off in offsets {
            if puzzles.len() >= limit {
                break;
            }

            file.seek(SeekFrom::Start(off))?;
            line_buf.clear();
            let mut reader = BufReader::new(&file);
            reader.read_line(&mut line_buf)?;

            if let Some(puzzle) = Puzzle::from_csv_line(line_buf.trim_end()) {
                if max_rating.map_or(true, |max| puzzle.rating <= max) {
                    puzzles.push(puzzle);
                }
            }
        }

        Ok(puzzles)
    }

    /// Load puzzles with offset (for pagination from the server).
    #[allow(dead_code)]
    pub fn load_theme_with_offset(
        &self,
        theme: &str,
        max_rating: Option<u16>,
        limit: usize,
        offset: usize,
    ) -> io::Result<Vec<Puzzle>> {
        let offsets = match self.theme_offsets.get(theme) {
            Some(o) => o,
            None => return Ok(Vec::new()),
        };

        let mut file = File::open(&self.path)?;
        let mut puzzles = Vec::with_capacity(limit.min(offsets.len()));
        let mut line_buf = String::new();
        let mut skipped = 0usize;

        for &off in offsets {
            if puzzles.len() >= limit {
                break;
            }

            file.seek(SeekFrom::Start(off))?;
            line_buf.clear();
            let mut reader = BufReader::new(&file);
            reader.read_line(&mut line_buf)?;

            if let Some(puzzle) = Puzzle::from_csv_line(line_buf.trim_end()) {
                if max_rating.map_or(true, |max| puzzle.rating <= max) {
                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }
                    puzzles.push(puzzle);
                }
            }
        }

        Ok(puzzles)
    }
}

/// Get the nth (0-indexed) CSV field without allocating.
fn nth_csv_field(line: &str, n: usize) -> Option<&str> {
    let mut start = 0;
    for _ in 0..n {
        start = line[start..].find(',')? + start + 1;
    }
    let end = line[start..].find(',').map_or(line.len(), |p| start + p);
    Some(&line[start..end])
}

/// Available tactic themes that map to Lichess theme tags.
pub const TACTIC_THEMES: &[(&str, &str)] = &[
    ("fork", "Fork / Double Attack"),
    ("pin", "Pin"),
    ("skewer", "Skewer"),
    ("discoveredAttack", "Discovered Attack"),
    ("mateIn1", "Mate in 1"),
    ("mateIn2", "Mate in 2"),
    ("mateIn3", "Mate in 3+"),
    ("backRankMate", "Back Rank Mate"),
    ("smotheredMate", "Smothered Mate"),
    ("hangingPiece", "Hanging Piece"),
    ("trappedPiece", "Trapped Piece"),
    ("deflection", "Deflection"),
    ("decoy", "Decoy"),
    ("overloading", "Overloading"),
    ("interference", "Interference"),
    ("sacrifice", "Sacrifice"),
    ("clearance", "Clearance"),
    ("quietMove", "Quiet Move"),
    ("xRayAttack", "X-Ray Attack"),
    ("zugzwang", "Zugzwang"),
    ("promotion", "Pawn Promotion"),
    ("underPromotion", "Underpromotion"),
    ("castling", "Castling"),
    ("enPassant", "En Passant"),
    ("exposedKing", "Exposed King"),
    ("kingsideAttack", "Kingside Attack"),
    ("queensideAttack", "Queenside Attack"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_field_iter() {
        let line = "abc,def,ghi,jkl";
        let mut iter = CsvFieldIter::new(line);
        assert_eq!(iter.next(), Some("abc"));
        assert_eq!(iter.next(), Some("def"));
        assert_eq!(iter.next(), Some("ghi"));
        assert_eq!(iter.next(), Some("jkl"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_nth_csv_field() {
        let line = "00sHx,fen_here,e8d7 a2e6,1760,80,83,72,mate mateIn2,url,opening";
        assert_eq!(nth_csv_field(line, 0), Some("00sHx"));
        assert_eq!(nth_csv_field(line, 3), Some("1760"));
        assert_eq!(nth_csv_field(line, 7), Some("mate mateIn2"));
    }

    #[test]
    fn test_puzzle_from_csv_line() {
        let line = "abc,r1bqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1,e2e4 d7d5,1200,80,90,50,fork short,url,opening";
        let p = Puzzle::from_csv_line(line).unwrap();
        assert_eq!(p.id, "abc");
        assert_eq!(p.rating, 1200);
        assert_eq!(p.solution_moves(), &["d7d5"]);
        assert_eq!(p.themes, vec!["fork", "short"]);
    }
}
