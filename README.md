# Cheshire Chess

A chess application that lives in your terminal. Play against 10 computer personalities, solve 5.8M puzzles, study 62 interactive lessons, compete online in game rooms, and train with mini-games — all from the command line. No browser, no GUI, no Electron. Works over SSH.

```
    +---+---+---+---+---+---+---+---+
  8 | R | N | B | Q | K | B | N | R |       Pieces in-game:
    +---+---+---+---+---+---+---+---+
  7 | p | p | p | p | p | p | p | p |       ▕▟✚▙▏  King
    +---+---+---+---+---+---+---+---+        ▀▀▀
  6 |   |   |   |   |   |   |   |   |
    +---+---+---+---+---+---+---+---+       ▕▟✠▙▏  Queen
  5 |   |   |   |   |   |   |   |   |        ◥■◤
    +---+---+---+---+---+---+---+---+
  4 |   |   |   |   |   |   |   |   |       ▕▟▆▙▏  Rook
    +---+---+---+---+---+---+---+---+        ▀▀▀
  3 |   |   |   |   |   |   |   |   |
    +---+---+---+---+---+---+---+---+         ▲    Bishop
  2 | p | p | p | p | p | p | p | p |        ▐▀▌
    +---+---+---+---+---+---+---+---+
  1 | R | N | B | Q | K | B | N | R |       ▂▅▅▅▃▃ Knight
    +---+---+---+---+---+---+---+---+       ▕▣▞ ▚▚▚
      a   b   c   d   e   f   g   h         ▀   ▚▚

                                               ⭘   Pawn
                                              ▜█▛
```

## Install

```bash
cargo install cheshire_chess
cheshire-chess
```

Or build from source:

```bash
git clone https://github.com/joshjetson/cheshire_chess.git
cd cheshire_chess
cargo build --release
./target/release/cheshire-chess
```

Requires Rust 1.75+. Linux audio needs `libasound2-dev` (`apt install pkg-config libasound2-dev`). Install without audio: `cargo install cheshire_chess --no-default-features`. macOS and Windows work out of the box.

## Features

### Play vs Computer — 10 Personalities

Play against a built-in chess engine with adjustable strength and famous player styles:

| Personality | Style |
|---|---|
| Beginner | Just learning, makes random mistakes |
| Casual | Plays for fun, occasional blunders |
| Club Player | Solid fundamentals |
| Strong Player | Finds tactical shots |
| Expert | Tournament strength, deep calculation |
| Bobby Fischer | Aggressive, precise, no mercy |
| Mikhail Tal | Wild sacrifices, chaotic attacks |
| Anatoly Karpov | Positional squeeze, quiet suffocation |
| Garry Kasparov | Dynamic, calculating, overwhelming force |
| Magnus Carlsen | Universal, patient, grinds you down |

The engine uses minimax with alpha-beta pruning, piece-square tables, and personality-specific evaluation weights. Each opponent has unique aggression, depth, and randomness settings.

### Tactics Training — 5.8 Million Puzzles

Pick from 27 tactic themes — fork, pin, skewer, discovered attack, mate in 1/2/3, back rank mate, smothered mate, and more. Puzzles are served from a central server — no download needed. Select a piece, see legal moves highlighted, play the solution.

### Study — 62 Interactive Lessons

Walk through annotated games and positions move by move:

**Famous Games** — The Immortal Game, Opera Game, Game of the Century, Deep Blue vs Kasparov, Fischer vs Spassky Game 6, Kasparov's Immortal, Fool's Mate, Scholar's Mate

**Openings (19)** — Italian, Sicilian, French, Queen's Gambit, Ruy Lopez, King's Indian, Caro-Kann, London, English, Scotch, King's Gambit, Scandinavian, Nimzo-Indian, Grünfeld, Slav, Petroff, Dutch, Pirc, Vienna

**Tactical Patterns** — Fork, Pin, Skewer, Discovered Attack, Deflection, Removing the Defender, Double Check, Overloading

**Checkmate Patterns** — Back Rank Mate, Smothered Mate, Arabian Mate, Anastasia's Mate, Boden's Mate, Epaulette Mate, Ladder Mate

**Endgame Theory** — K+Q vs K, K+R vs K, Opposition, Lucena Position, Philidor Position, Rule of the Square, Triangulation, Passed Pawns, Two Bishops Mate

**Pawn Structures** — Isolated Queen Pawn, Doubled Pawns, Passed Pawns, Backward Pawns, Pawn Chains, Pawn Majority

### Mini-Games

- **Knight's Tour** — move a knight to visit all 64 squares. Tests visualization and planning.
- **Color Quiz** — flash a square name, answer light or dark. Builds instant board awareness. Tracks score, streak, and best streak.
- **Blindfold Mode** — coming soon.

### Live Multiplayer

Select **Go Online** to connect to the central game server. No ports to open, no setup.

- **Game Rooms** — create or join rooms. Name your room. See who's online.
- **Game Tables** — sit down at a table, another player joins, game starts automatically. Spectators can watch.
- **Room Chat** — everyone shares one chat. Talk to players and spectators.
- **Rematch** — after a game, press `r` to rematch with swapped colors.

### Synthesized Audio

All sounds are generated mathematically — no audio files. Warm, filtered tones for every event:

- Piece move, capture, check, checkmate
- Correct/wrong puzzle answers, hints
- Login/exit sounds

Every sound is fully customizable in **Settings > Sound Settings**: waveform (sine/triangle/saw/square), ADSR envelope, LFO modulation, low-pass filter cutoff. Preview and save.

### Custom Pieces

Draw your own chess pieces with a built-in canvas editor using 200+ Unicode block characters. Or edit `data/custom_pieces.txt` directly. Pieces persist across sessions.

### Controls

Board always on the left, context panel on the right. **Tab** switches focus.

| Key | Action |
|---|---|
| `Tab` | Switch focus between board and panel |
| `hjkl` / arrows | Navigate within the focused pane |
| `Enter` | Select / activate |
| `Esc` | Deselect / go back |
| `Ctrl+C` | Quit from anywhere |

## Architecture

- **Rust** — ratatui + crossterm for the terminal UI
- **Bitboard engine** — 64-bit position representation, alpha-beta search with piece-square tables
- **WebSocket networking** — central game server via WSS through Caddy
- **Synthesized audio** — runtime-generated sounds with ADSR, LFO, and filtering
- **Server-side puzzles** — 5.8M Lichess puzzles served from `chess.virtualraremedia.com`

```
src/
├── main.rs        — event loop, terminal setup
├── app.rs         — state machine, screen/key handling
├── board.rs       — bitboard position, FEN, move gen, check/mate
├── engine.rs      — chess AI: minimax, alpha-beta, personalities
├── ui.rs          — ratatui rendering for all screens
├── lessons.rs     — 62 study lessons baked into the binary
├── minigames.rs   — knight's tour, color quiz, blindfold
├── puzzle.rs      — Lichess puzzle indexing and on-demand loading
├── audio.rs       — synthesized sound engine, ADSR, LFO, filter
├── settings.rs    — persistent settings and synth params
├── canvas.rs      — piece editor, shape palette, save/load
├── server.rs      — embedded WebSocket game server
├── net.rs         — client networking, channel bridge
├── protocol.rs    — JSON message types (client <-> server)
├── tracker.rs     — tracker client for server discovery
├── identity.rs    — client identity hash generation
└── progress.rs    — puzzle completion tracking
```

## License

MIT
