mod test_cli {
    pub use crate::cli::{CliError, CommonArgs};

    pub fn resolve_base_url(common: &CommonArgs) -> String {
        common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", common.host, common.port))
    }

    pub fn resolve_token(common: &CommonArgs) -> Result<String, CliError> {
        common
            .token
            .clone()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| CliError::usage("no bearer token"))
    }
}

#[path = "../src/cli/mod.rs"]
mod cli;

use cli::tui::console::{ConsoleFilter, ConsoleView, LOCAL_SCROLLBACK_LIMIT};
use msc_infrastructure::console_buffer::ConsoleLine;

fn line(source: &str, level: Option<&str>, text: &str) -> ConsoleLine {
    ConsoleLine {
        ts: text.to_string(),
        source: source.to_string(),
        level: level.map(str::to_string),
        text: text.to_string(),
    }
}

#[test]
fn console_view_keeps_bounded_history_and_distinguishes_filters_and_selection() {
    let mut view = ConsoleView::from_lines([
        line("server", None, "started"),
        line("plugin", None, "Plugin loaded"),
        line("controller", Some("warning"), "backup failed"),
        line("command", None, "> save-all"),
    ]);

    assert_eq!(view.visible_lines().len(), 4);
    view.set_filter(ConsoleFilter::Warnings);
    assert_eq!(view.visible_lines().len(), 1);
    view.set_filter(ConsoleFilter::Commands);
    assert_eq!(view.visible_lines().len(), 1);
    view.set_filter(ConsoleFilter::Custom("plugin".to_string()));
    assert_eq!(view.visible_lines().len(), 1);

    view.set_filter(ConsoleFilter::All);
    view.move_selection(-1);
    view.toggle_selection_anchor();
    view.move_selection(-1);
    assert!(view.selected_text().contains("Plugin loaded"));

    let bounded_lines = (0..(LOCAL_SCROLLBACK_LIMIT + 10))
        .map(|index| line("server", None, &format!("line-{index}")))
        .collect::<Vec<_>>();
    let bounded_view = ConsoleView::from_lines(bounded_lines);
    assert_eq!(bounded_view.visible_lines().len(), LOCAL_SCROLLBACK_LIMIT);
}

#[test]
fn raw_command_input_is_separate_from_palette_selection() {
    let mut view = ConsoleView::from_lines([]);
    view.begin_command();
    for character in "say hello".chars() {
        view.push_input(character);
    }
    assert_eq!(view.take_command().as_deref(), Some("say hello"));

    view.begin_palette();
    assert_eq!(view.selected_palette_command(), "time set day");
    view.move_palette(6);
    assert_eq!(view.selected_palette_command(), "reload");
}
