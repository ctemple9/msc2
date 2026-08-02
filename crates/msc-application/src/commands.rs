//! Command delivery for the Java lifecycle slice.
//!
//! Autocomplete and command catalog behavior belongs to `msc-domain`; this
//! module only covers getting a validated command onto the running process'
//! stdin.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandInputError {
    MissingCommand,
    EmptyCommand,
}

impl CommandInputError {
    pub fn code(self) -> &'static str {
        match self {
            Self::MissingCommand => "missing_command",
            Self::EmptyCommand => "missing_command",
        }
    }
}

impl fmt::Display for CommandInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "missing command"),
            Self::EmptyCommand => write!(f, "empty command"),
        }
    }
}

impl std::error::Error for CommandInputError {}

pub fn validate_api_command(raw: Option<&str>) -> Result<String, CommandInputError> {
    let raw = raw.ok_or(CommandInputError::MissingCommand)?;
    let command = raw.trim();
    if command.is_empty() {
        Err(CommandInputError::EmptyCommand)
    } else {
        Ok(command.to_string())
    }
}

pub fn stdin_payload(command: &str) -> Vec<u8> {
    if command.ends_with('\n') {
        command.as_bytes().to_vec()
    } else {
        let mut payload = Vec::with_capacity(command.len() + 1);
        payload.extend_from_slice(command.as_bytes());
        payload.push(b'\n');
        payload
    }
}
