mod app;
mod audio;
mod board;
mod canvas;
mod identity;
mod lessons;
mod net;
mod progress;
mod protocol;
mod puzzle;
mod server;
mod settings;
mod tracker;
mod ui;

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::App;

fn main() -> io::Result<()> {
    let quit = Arc::new(AtomicBool::new(false));
    let quit_sig = quit.clone();
    ctrlc_handler(quit_sig);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &quit);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn ctrlc_handler(quit: Arc<AtomicBool>) {
    let _ = unsafe {
        libc::signal(libc::SIGINT, sigint_handler as libc::sighandler_t)
    };
    QUIT_FLAG.store(Box::into_raw(Box::new(quit)) as usize, Ordering::SeqCst);
}

static QUIT_FLAG: AtomicUsize = AtomicUsize::new(0);

extern "C" fn sigint_handler(_sig: libc::c_int) {
    let ptr = QUIT_FLAG.load(Ordering::SeqCst);
    if ptr != 0 {
        let flag = unsafe { &*(ptr as *const AtomicBool) };
        flag.store(true, Ordering::SeqCst);
    }
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, quit: &AtomicBool) -> io::Result<()> {
    let data_dir = Path::new("data");
    let mut app = App::new(data_dir);
    app.play_sound(|a, s| a.play_login(s));

    let puzzle_path = Path::new("data/lichess_puzzles.csv");
    if puzzle_path.exists() {
        match app.build_index(puzzle_path) {
            Ok(count) => {
                app.message = format!("{count} puzzles indexed. hjkl/arrows to navigate, Enter to select");
            }
            Err(e) => {
                app.message = format!("Failed to index puzzles: {e}");
            }
        }
    }

    while app.running && !quit.load(Ordering::Relaxed) {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        app.poll_network();

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
    }

    app.play_sound(|a, s| a.play_exit(s));
    std::thread::sleep(Duration::from_millis(400)); // let exit sound finish

    Ok(())
}
