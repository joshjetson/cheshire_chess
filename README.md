# Cheshire Chess

A chess application that lives in your terminal. Practice tactics, play live games, and hang out in game rooms — all from the command line. No browser, no GUI, no Electron. Works over SSH.

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
```

Or build from source:

```bash
git clone https://github.com/joshjetson/cheshire_chess.git
cd cheshire_chess
cargo build --release
```

### Puzzles

Download the Lichess puzzle database for tactics training:

```bash
mkdir -p data
curl -L -o data/lichess_db_puzzle.csv.zst https://database.lichess.org/lichess_db_puzzle.csv.zst
zstd -d data/lichess_db_puzzle.csv.zst -o data/lichess_puzzles.csv
```

## Usage

```bash
cheshire-chess
```

### Controls

| Key | Action |
|---|---|
| `hjkl` / arrows | Navigate menus and board |
| `Enter` | Select / place piece / submit move |
| `Tab` | Toggle chat (online) / shape picker (canvas) |
| `Esc` | Go back |
| `Ctrl+C` | Quit from anywhere |
| `q` | Quit / back |

## What It Does

- **5.8 million puzzles** from the Lichess database — forks, pins, skewers, mates, and more
- **Live multiplayer** — create game rooms, play opponents, spectate, and chat
- **Peer-to-peer** — every client is also a server. No central game server required
- **Internet discovery** — players find each other automatically through a tracker service
- **Custom pieces** — draw your own piece art with a built-in canvas editor
- **Runs anywhere** — pure terminal UI, works in any terminal emulator, over SSH, on any OS

## Features

### Tactics Training

Pick a tactic theme from 27 categories — fork, pin, skewer, mate in 1/2/3, back rank mate, smothered mate, and more. Each puzzle tells you which color to play. Select a piece, see its legal moves highlighted, and play the solution.

Correct moves advance the puzzle. Wrong moves let you try again. `H` gives a hint. Puzzles are loaded on demand from the 5.8M Lichess database — no wait time.

### Live Multiplayer

Select **Go Online** from the menu. Your app starts hosting automatically — no separate server to run.

**Game Rooms** — browse existing rooms or create your own. Each room is a chess club:

- **Game Tables** — anyone in the room can create a table. Another player joins to start a game. Spectators can watch any table.
- **Room Chat** — everyone in the room shares one chat. Talk to players, spectators, whoever.
- **Internet Discovery** — when you go online, your server registers with a tracker at `chess.virtualraremedia.com`. Other players see you in their room browser and can connect directly.

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

Draw your own chess pieces using Unicode block characters. Select **Piece Canvas** from the menu:

1. Pick a piece type (King, Queen, Rook, Bishop, Knight, Pawn)
2. Choose from 200+ shapes — block elements, box drawing, geometric shapes, symbols
3. Draw on a 7x3 grid with live preview
4. Save — pieces persist in `data/custom_pieces.txt` and load automatically

You can also edit `data/custom_pieces.txt` directly:

```
# King
5
 ▕▟✚▙▏
  ▀▀▀
```

Spaces are transparent. Everything else renders in the piece color.

### Board Theme

Cheshire Cat purple — soft lavender light squares, deep purple dark squares, pink cursor highlight.

## Architecture

- **Rust** with `ratatui` + `crossterm` for the terminal UI
- **Bitboard** chess engine — 64-bit integers for position representation and move generation
- **WebSocket** networking — every client embeds a server, games connect peer-to-peer
- **Tracker** service at `chess.virtualraremedia.com` for internet player discovery

```
src/
├── main.rs        — event loop, terminal setup
├── app.rs         — state machine, screen/key handling
├── board.rs       — bitboard position, FEN, move gen, check/mate
├── ui.rs          — ratatui rendering for all screens
├── puzzle.rs      — Lichess CSV indexing, on-demand loading
├── canvas.rs      — piece editor, shape palette, save/load
├── server.rs      — embedded WebSocket game server
├── net.rs         — client networking, channel bridge
├── protocol.rs    — JSON message types (client <-> server)
└── tracker.rs     — tracker client for discovery
```

## License

MIT
