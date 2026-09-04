#[allow(dead_code)]
#[path = "../src/cli/mod.rs"]
mod cli;

use std::cell::RefCell;
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use cli::tui::TerminalControl;
use cli::tui::TerminalGuard;
use cli::tui::app::App;
use cli::tui::layout::{LayoutMode, layout_mode};
use cli::tui::render;

#[derive(Clone)]
struct RecordingTerminalControl {
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl RecordingTerminalControl {
    fn new(events: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self { events }
    }
}

impl TerminalControl for RecordingTerminalControl {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        self.events.borrow_mut().push("enable raw");
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        self.events
            .borrow_mut()
            .push("enter alternate / hide cursor");
        Ok(())
    }

    fn restore(&mut self) -> io::Result<()> {
        self.events
            .borrow_mut()
            .extend(["show cursor", "leave alternate", "disable raw"]);
        Ok(())
    }
}

#[test]
fn terminal_guard_restores_every_terminal_state_after_return_error_and_panic() {
    let normal = Rc::new(RefCell::new(Vec::new()));
    {
        let mut guard =
            TerminalGuard::enter(RecordingTerminalControl::new(normal.clone())).unwrap();
        guard.restore().unwrap();
    }
    assert_restore_trace(&normal.borrow());

    let error = Rc::new(RefCell::new(Vec::new()));
    let result = run_until_error(error.clone());
    assert!(result.is_err());
    assert_restore_trace(&error.borrow());

    let resize = Rc::new(RefCell::new(Vec::new()));
    {
        let _guard = TerminalGuard::enter(RecordingTerminalControl::new(resize.clone())).unwrap();
        let mut app = App::new("test-host");
        app.resize(100, 30);
    }
    assert_restore_trace(&resize.borrow());

    let panic_trace = Rc::new(RefCell::new(Vec::new()));
    let panic_result = panic::catch_unwind(AssertUnwindSafe({
        let panic_trace = panic_trace.clone();
        move || {
            let _guard = TerminalGuard::enter(RecordingTerminalControl::new(panic_trace)).unwrap();
            panic!("simulated event-loop panic");
        }
    }));
    assert!(panic_result.is_err());
    assert_restore_trace(&panic_trace.borrow());
}

#[test]
fn shell_renders_every_required_wide_medium_and_small_structure_without_clipping() {
    let (wide, _) = render_at(140, 42, |_| {});
    for marker in [
        "Host: test-host",
        "SERVER CONTROLS",
        "SERVER IDENTITY",
        "Overview",
        "Players",
        "Worlds",
        "Performance",
        "Components",
        "Settings",
        "Files",
        "Connection and live stats",
        "Server Health",
        "Activity",
        "CONSOLE",
    ] {
        assert!(
            wide.contains(marker),
            "wide shell clipped or omitted {marker:?}:\n{wide}"
        );
    }
    let (medium, _) = render_at(100, 30, |_| {});
    for marker in [
        "SERVER CONTROLS",
        "Section: Overview",
        "Rail: shown",
        "Console: shown",
        "CONSOLE",
    ] {
        assert!(
            medium.contains(marker),
            "medium shell omitted {marker:?}:\n{medium}"
        );
    }

    let (collapsed_medium, _) = render_at(100, 30, |app| {
        app.resize(100, 30);
        app.handle_key(KeyCode::Char('r'));
        app.handle_key(KeyCode::Char('c'));
    });
    assert!(collapsed_medium.contains("Rail: hidden"));
    assert!(collapsed_medium.contains("Console: hidden"));
    assert!(!collapsed_medium.contains("No console data loaded."));

    let (small, _) = render_at(70, 20, |_| {});
    for marker in [
        "FOCUSED VIEW",
        "Host: test-host",
        "[s] sections",
        "[c] console",
        "[?] help",
    ] {
        assert!(
            small.contains(marker),
            "small shell omitted {marker:?}:\n{small}"
        );
    }
}

#[test]
fn resize_and_keys_keep_focus_and_every_small_surface_reachable() {
    assert_eq!(layout_mode(Rect::new(0, 0, 140, 42)), LayoutMode::Wide);
    assert_eq!(layout_mode(Rect::new(0, 0, 100, 30)), LayoutMode::Medium);
    assert_eq!(layout_mode(Rect::new(0, 0, 70, 20)), LayoutMode::Small);

    let (sections, _) = render_at(70, 20, |app| {
        app.resize(70, 20);
        app.handle_key(KeyCode::Char('s'));
    });
    assert!(sections.contains("SECTIONS"));
    assert!(sections.contains("[j/k] choose"));

    let (console, _) = render_at(70, 20, |app| {
        app.resize(70, 20);
        app.handle_key(KeyCode::Char('c'));
    });
    assert!(console.contains("› CONSOLE"));
    assert!(console.contains("No console data loaded."));

    let (help, _) = render_at(70, 20, |app| {
        app.resize(70, 20);
        app.handle_key(KeyCode::Char('?'));
    });
    assert!(help.contains("› KEYBOARD HELP"));
    assert!(help.contains("[1-7] server sections"));
}

fn run_until_error(events: Rc<RefCell<Vec<&'static str>>>) -> Result<(), ()> {
    let _guard = TerminalGuard::enter(RecordingTerminalControl::new(events)).unwrap();
    Err(())
}

fn assert_restore_trace(events: &[&str]) {
    assert_eq!(
        events,
        [
            "enable raw",
            "enter alternate / hide cursor",
            "show cursor",
            "leave alternate",
            "disable raw",
        ]
    );
}

fn render_at(width: u16, height: u16, configure: impl FnOnce(&mut App)) -> (String, App) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new("test-host");
    configure(&mut app);
    terminal
        .draw(|frame| render::render(frame, &mut app))
        .unwrap();
    (buffer_text(terminal.backend().buffer()), app)
}

fn buffer_text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
