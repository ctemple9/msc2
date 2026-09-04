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

use cli::tui::agent::{AgentIntent, AgentState, AgentSurface};
use cli::tui::app::App;
use cli::tui::app_settings::{AppSettingsIntent, AppSettingsState};
use cli::tui::handbook::{HandbookState, HelpCatalog, HelpCatalogEntry, HelpTopic};
use crossterm::event::KeyCode;

#[test]
fn pairing_input_stays_in_memory_and_requires_a_complete_code() {
    let mut agent = AgentState {
        surface: AgentSurface::Pairing,
        ..AgentState::default()
    };
    assert_eq!(
        agent.handle_key(KeyCode::Char('c')),
        Some(AgentIntent::BeginPairing)
    );
    assert_eq!(agent.handle_key(KeyCode::Char('p')), None);
    for character in "pair_example".chars() {
        agent.handle_key(KeyCode::Char(character));
    }
    assert_eq!(
        agent.handle_key(KeyCode::Enter),
        Some(AgentIntent::ExchangePairing("pair_example".to_string()))
    );
    assert!(agent.pairing_code.is_none());
}

#[test]
fn handbook_search_and_related_topics_keep_the_served_content_shape() {
    let mut handbook = HandbookState {
        catalog: Some(HelpCatalog {
            topics: vec![
                HelpCatalogEntry {
                    help_id: "handbook.ram".to_string(),
                    title: "RAM and performance".to_string(),
                    category: "server".to_string(),
                },
                HelpCatalogEntry {
                    help_id: "handbook.networking".to_string(),
                    title: "Networking basics".to_string(),
                    category: "network".to_string(),
                },
            ],
        }),
        topic: Some(HelpTopic {
            help_id: "handbook.ram".to_string(),
            title: "RAM and performance".to_string(),
            subtitle: None,
            analogy: None,
            body: "Keep room for Java.".to_string(),
            category: "server".to_string(),
            related_ids: vec!["handbook.networking".to_string()],
            sections: Vec::new(),
        }),
        ..HandbookState::default()
    };
    handbook.query = "network".to_string();
    assert_eq!(handbook.filtered_topics()[0].help_id, "handbook.networking");
    assert_eq!(handbook.related_titles(), vec!["Networking basics"]);
}

#[test]
fn client_reset_is_not_a_host_reset() {
    let mut app = App::new("host-a:48001");
    app.set_note("local reminder");
    app.handle_key(KeyCode::Char(','));
    assert!(!app.handle_key(KeyCode::Char('c')));
    assert!(app.notes_for_selected_server().is_none());
    assert!(app.overview().servers.is_empty());

    let mut settings = AppSettingsState::default();
    settings.handle_key(KeyCode::Char('3'));
    for character in "RESET AGENT".chars() {
        settings.handle_key(KeyCode::Char(character));
    }
    assert_eq!(
        settings.handle_key(KeyCode::Enter),
        Some(AppSettingsIntent::HostReset {
            mode: "everything".to_string(),
            confirmation: "RESET AGENT".to_string(),
        })
    );
}
