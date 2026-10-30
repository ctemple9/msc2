//! Matches the desktop chatFeed parser over the selected server console tail.
use crossterm::style::Color;
pub(super) struct Message {
    pub text: String,
    pub color: Color,
}
fn message(text: String, color: Color) -> Option<Message> {
    Some(Message { text, color })
}
pub(super) fn parse(raw: &str) -> Option<Message> {
    let payload = raw.rsplit_once("]: ").map_or(raw, |(_, text)| text);
    let payload = payload
        .strip_prefix("[Not Secure] ")
        .unwrap_or(payload)
        .trim();
    if let Some(chat) = payload.strip_prefix('<') {
        let (name, text) = chat.split_once('>')?;
        if !name.trim().is_empty() && !text.trim().is_empty() {
            return message(format!("{} {}", name.trim(), text.trim()), Color::White);
        }
    }
    for marker in [
        " has made the advancement ",
        " has completed the challenge ",
        " has reached the goal ",
    ] {
        if let Some((name, text)) = payload.split_once(marker)
            && !name.trim().is_empty()
            && name.trim().len() < 40
        {
            let text = text.trim();
            let text = text
                .strip_prefix('[')
                .and_then(|t| t.strip_suffix(']'))
                .unwrap_or(text);
            return message(format!("{} earned {}", name.trim(), text), Color::Yellow);
        }
    }
    for ending in [" joined the game", " left the game"] {
        if let Some(name) = payload.strip_suffix(ending) {
            return message(format!("{name}{ending}"), Color::DarkGrey);
        }
    }
    for (marker, action) in [
        ("Player connected: ", "connected"),
        ("Player disconnected: ", "disconnected"),
    ] {
        if let Some((_, rest)) = payload.split_once(marker) {
            return message(
                format!("{} {action}", rest.split(',').next().unwrap_or(rest).trim()),
                Color::DarkGrey,
            );
        }
    }
    None
}
