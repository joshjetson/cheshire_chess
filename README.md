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

### From crates.io

```bash
cargo install cheshire_chess
cheshire-chess
```

### From source

```bash
git clone https://github.com/joshjetson/cheshire_chess.git
cd cheshire_chess
cargo build --release
./target/release/cheshire-chess
```

Requires Rust 1.75 or newer.

### Linux audio dependencies

Audio requires ALSA development libraries on Linux:

```bash
# Debian/Ubuntu
sudo apt install pkg-config libasound2-dev

# Fedora
sudo dnf install alsa-lib-devel

# Arch
sudo pacman -S alsa-lib
```

To install without audio (no system deps needed):

```bash
cargo install cheshire_chess --no-default-features
```

macOS and Windows work out of the box — no extra dependencies.

### Puzzles (optional)

Download the Lichess puzzle database for tactics training (~300MB download, ~1GB uncompressed):

```bash
mkdir -p data
curl -L -o data/lichess_db_puzzle.csv.zst https://database.lichess.org/lichess_db_puzzle.csv.zst
zstd -d data/lichess_db_puzzle.csv.zst -o data/lichess_puzzles.csv
rm data/lichess_db_puzzle.csv.zst
```

Place the `data/` directory wherever you run the app from.

### Controls

The board is always visible on the left. Everything else happens in the right pane.

| Key | Action |
|---|---|
| `hjkl` / arrows | Move cursor on the board |
| `Enter` | Select piece / place piece / submit move |
| `Tab` | Cycle menu items / toggle chat |
| `Space` | Activate selected menu item |
| `1-5` | Quick-jump to menu items |
| `Esc` | Deselect / go back |
| `Ctrl+C` | Quit from anywhere |
| `q` | Quit |

On the main screen you can pick up and move pieces freely — no rules enforced. Select a piece with Enter, move cursor to destination, Enter again to place it.

## What It Does

- **5.8 million puzzles** from the Lichess database — forks, pins, skewers, mates, and more
- **Live multiplayer** — create game rooms, play opponents, spectate, and chat
- **Peer-to-peer** — every client is also a server. No central game server required
- **Internet discovery** — players find each other automatically through a tracker service
- **Synthesized audio** — all sounds generated mathematically at runtime, fully customizable
- **Sound designer** — tweak waveform, ADSR envelope, LFO, and filter per sound event
- **Custom pieces** — draw your own piece art with a built-in canvas editor
- **Settings** — player name, sound parameters, and piece canvas all in one place
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

### Settings

Access from the main menu. Includes:

- **Player Name** — set your display name for online play
- **Sound Settings** — per-event synth controls: waveform (sine/triangle/saw/square), ADSR envelope, LFO rate and depth, low-pass filter cutoff. Preview and save.
- **Piece Canvas** — draw custom pieces (see below)

All settings persist to `data/settings.json`.

### Custom Pieces

Draw your own chess pieces using Unicode block characters. Open from **Settings > Piece Canvas**:

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
├── audio.rs       — synthesized sound engine, ADSR, LFO, filter
├── settings.rs    — persistent settings, synth params, save/load
├── server.rs      — embedded WebSocket game server
├── net.rs         — client networking, channel bridge
├── protocol.rs    — JSON message types (client <-> server)
└── tracker.rs     — tracker client for discovery
```

## License

MIT
