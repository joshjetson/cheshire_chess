# Cheshire Chess

A chess application that lives in your terminal. Practice tactics, play live games, and hang out in game rooms — all from the command line. No browser, no GUI, no Electron. Works over SSH.

```
  Cheshire Chess
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│8 ▕▟▆▙▏ ▂▅▅▅▃▃   ▲    ▕▟✠▙▏ ▕▟✚▙▏   ▲   ▂▅▅▅▃▃ ▕▟▆▙▏│
│   ▀▀▀  ▕▣▞ ▚▚▚ ▐▀▌    ◥■◤    ▀▀▀   ▐▀▌  ▕▣▞ ▚▚▚  ▀▀▀ │
│         ▀   ▚▚                              ▀   ▚▚        │
│7  ⭘     ⭘     ⭘     ⭘     ⭘     ⭘     ⭘     ⭘  │
│  ▜█▛   ▜█▛   ▜█▛   ▜█▛   ▜█▛   ▜█▛   ▜█▛   ▜█▛  │
│                                                             │
│6                                                            │
│                                                             │
│                                                             │
│5                                                            │
│                                                             │
│                                                             │
│   a     b     c     d     e     f     g     h               │
└─────────────────────────────────────────────────────────────┘
```

## What It Does

- **5.8 million puzzles** from the Lichess database — forks, pins, skewers, mates, and more
- **Live multiplayer** — create game rooms, play opponents, spectate, and chat
- **Runs anywhere** — pure terminal UI, works in any terminal emulator, over SSH, on any OS
- **Peer-to-peer** — every client is a server. No central game server required
- **Custom pieces** — draw your own piece art with a built-in canvas editor

## Install

```bash
git clone https://github.com/youruser/cheshire_chess.git
cd cheshire_chess
cargo build --release
```

The binary is at `target/release/cheshire-chess`.

### Puzzles

Download the Lichess puzzle database for tactics training:

```bash
mkdir -p data
curl -L -o data/lichess_db_puzzle.csv.zst https://database.lichess.org/lichess_db_puzzle.csv.zst
zstd -d data/lichess_db_puzzle.csv.zst -o data/lichess_puzzles.csv
```

## Usage

```bash
cargo run --release
```

### Controls

| Key | Action |
|---|---|
| `hjkl` / arrows | Navigate menus and board |
| `Enter` | Select menu item / select piece / submit move |
| `Tab` | Open chat (online) / shape picker (canvas) |
| `Esc` | Go back |
| `Ctrl+C` | Quit from anywhere |
| `q` | Quit / back |

## Features

### Tactics Training

Pick a tactic theme from 27 categories — fork, pin, skewer, mate in 1/2/3, back rank mate, smothered mate, and more. Each puzzle shows the board and tells you which color to play. Select a piece, see its legal moves highlighted, and play the solution.

```
Puzzle 1/200 (rating: 1450) — Play as White. Select a piece.
```

Correct moves advance the puzzle. Wrong moves let you try again. `H` gives a hint. Puzzles are loaded on demand from the 5.8M Lichess database — no wait time.

### Live Multiplayer

Select **Go Online** from the menu. Your app starts hosting automatically on port 7878 — no separate server needed.

**Game Rooms** — browse existing rooms or create your own. Each room is a chess club:

- **Game Tables** — anyone in the room can create a table. Another player joins to start a game. Spectators can watch.
- **Room Chat** — everyone in the room shares one chat. Talk to players, spectators, whoever.
- **Spectating** — join any table to watch a game in progress. See every move in real time.

```
┌─ Josh's Room ──────────┐┌─ Chat ──────────────────┐
│ > Table 1: josh vs     ││  josh created the room  │
│   alex [playing]       ││  alex joined            │
│   Table 2: (open) vs   ││  josh: ready?           │
│   (open) [waiting]     ││  alex: let's go         │
│                        ││                          │
├─ 3 players ────────────┤│  > nice fork!_          │
│   josh (you) [playing] ││                          │
│   alex [playing]       │└──────────────────────────┘
│   sam [watching]       │
└────────────────────────┘
```

### Custom Pieces

The built-in piece canvas lets you draw your own chess pieces using Unicode block characters. Select **Piece Canvas** from the menu:

1. Pick a piece type (King, Queen, Rook, Bishop, Knight, Pawn)
2. Choose from 200+ shapes — block elements, box drawing, geometric shapes, symbols
3. Draw on a 7x3 grid — stamp characters, erase cells, preview on both square colors
4. Save — your pieces persist in `data/custom_pieces.txt` and load automatically

You can also edit `data/custom_pieces.txt` directly. Each piece is a type number followed by 3 lines of 7 characters:

```
# King
5
 ▕▟✚▙▏
  ▀▀▀
```

Spaces are transparent (show the board square color). Everything else renders in the piece color.

### Board Theme

Cheshire Cat purple — soft lavender light squares, deep purple dark squares, pink cursor highlight. Pieces get a subtle glow effect (light shading around edges).

## Architecture

- **Rust** with `ratatui` + `crossterm` for the terminal UI
- **Bitboard** chess engine — 64-bit integers for fast position representation and move generation
- **WebSocket** networking — every client embeds a server, games connect peer-to-peer
- **Tracker** service for internet discovery — players register and find each other through a central phone book at `chess.virtualraremedia.com`

### Project Structure

```
src/
├── main.rs        — event loop, terminal setup
├── app.rs         — state machine, all screen/key handling
├── board.rs       — bitboard position, FEN, move gen, check/mate detection
├── ui.rs          — ratatui rendering for all screens
├── puzzle.rs      — Lichess CSV indexing, on-demand puzzle loading
├── canvas.rs      — piece editor state, shape palette, save/load
├── server.rs      — embedded WebSocket game server
├── net.rs         — client networking, channel bridge to event loop
├── protocol.rs    — JSON message types (client <-> server)
├── tracker.rs     — tracker client for internet discovery
└── bin/
    ├── server.rs  — standalone dedicated server
    └── tracker.rs — discovery tracker service
```

## License

MIT
