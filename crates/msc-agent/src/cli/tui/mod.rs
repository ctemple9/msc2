//! Interactive terminal lifecycle. This module owns only terminal state and
//! presentation; authentication and all management behavior remain agent API
//! responsibilities.

pub mod activity;
pub mod app;
pub mod backups;
pub mod components;
pub mod confirm;
pub mod console;
pub mod layout;
pub mod overview;
pub mod performance;
pub mod players;
pub mod render;
pub mod session;
pub mod transport;
pub mod worlds;

use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::cli::{CliError, CommonArgs};

use self::app::App;

/// The small interface that lets lifecycle tests prove every terminal mode is
/// restored without making the test process enter raw mode.
pub trait TerminalControl {
    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn enter_alternate_screen(&mut self) -> io::Result<()>;
    fn restore(&mut self) -> io::Result<()>;
}

#[derive(Debug, Default)]
pub struct CrosstermTerminalControl;

impl TerminalControl for CrosstermTerminalControl {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        execute!(io::stdout(), EnterAlternateScreen, Hide)
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut first_error = execute!(io::stdout(), Show, LeaveAlternateScreen).err();
        if let Err(error) = disable_raw_mode()
            && first_error.is_none()
        {
            first_error = Some(error);
        }
        first_error.map_or(Ok(()), Err)
    }
}

/// Restores raw mode, cursor visibility, and the alternate screen exactly once.
/// Its `Drop` implementation covers ordinary early returns; `run` also calls it
/// before resuming a panic so the panic report lands in the normal terminal.
pub struct TerminalGuard<C: TerminalControl> {
    control: C,
    active: bool,
}

impl<C: TerminalControl> TerminalGuard<C> {
    pub fn enter(mut control: C) -> io::Result<Self> {
        control.enable_raw_mode()?;
        if let Err(error) = control.enter_alternate_screen() {
            let _ = control.restore();
            return Err(error);
        }
        Ok(Self {
            control,
            active: true,
        })
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        self.active = false;
        self.control.restore()
    }
}

impl<C: TerminalControl> Drop for TerminalGuard<C> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub fn run(common: CommonArgs) -> Result<(), CliError> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
        .map_err(|error| CliError::internal(format!("failed to prepare terminal: {error}")))?;
    let mut guard = TerminalGuard::enter(CrosstermTerminalControl)
        .map_err(|error| CliError::internal(format!("failed to enter terminal mode: {error}")))?;
    let mut app = App::from_common(&common);

    let loop_result = panic::catch_unwind(AssertUnwindSafe(|| event_loop(&mut terminal, &mut app)));
    let restore_result = guard.restore();

    match loop_result {
        Ok(Ok(())) => restore_result
            .map_err(|error| CliError::internal(format!("failed to restore terminal: {error}"))),
        Ok(Err(error)) => Err(CliError::internal(format!(
            "terminal event loop failed: {error}"
        ))),
        Err(payload) => {
            // The terminal is already restored above, before the original panic
            // reaches Rust's panic hook and writes its message.
            panic::resume_unwind(payload);
        }
    }
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render::render(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if app.handle_key(key.code) {
                    return Ok(());
                }
            }
            Event::Resize(width, height) => {
                app.resize(width, height);
                terminal.autoresize()?;
            }
            _ => {}
        }
    }
}
