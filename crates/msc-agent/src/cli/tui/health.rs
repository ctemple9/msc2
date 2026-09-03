//! Health cards and startup repair actions from the existing diagnostic API.

use crossterm::event::KeyCode;
use msc_api::dto::{
    HealthProblemsResponseDto, HealthRepairRequestDto, HealthRepairResultDto, HealthResponseDto,
};

use super::transport::SharedClient;
use crate::cli::CliError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthMutation {
    Repair { problem_id: String, action: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthIntent {
    Confirm(HealthMutation),
}

#[derive(Debug, Clone, Default)]
pub struct HealthState {
    pub health: Option<HealthResponseDto>,
    pub problems: Option<HealthProblemsResponseDto>,
    pub selected_problem: usize,
    pub detail_open: bool,
    pub loaded: bool,
    pub error: Option<String>,
}

impl HealthState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let health = client.get_json("/v1/health").await?;
        let problems = client.get_json("/v1/health/problems").await?;
        Ok(Self {
            health: Some(health),
            problems: Some(problems),
            loaded: true,
            ..Self::default()
        })
    }

    pub fn selected_problem(&self) -> Option<&msc_api::dto::StartupProblemDto> {
        self.problems.as_ref()?.problems.get(self.selected_problem)
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<HealthIntent> {
        if self.detail_open {
            return match key {
                KeyCode::Esc => {
                    self.detail_open = false;
                    None
                }
                KeyCode::Char('1'..='4') => {
                    let action_index = match key {
                        KeyCode::Char(value) => value.to_digit(10).unwrap_or(1) as usize - 1,
                        _ => unreachable!(),
                    };
                    let problem = self.selected_problem()?;
                    let action = problem.available_actions.get(action_index)?.clone();
                    Some(HealthIntent::Confirm(HealthMutation::Repair {
                        problem_id: problem.id.clone(),
                        action,
                    }))
                }
                _ => None,
            };
        }
        match key {
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Enter => self.detail_open = self.selected_problem().is_some(),
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    fn move_selection(&mut self, offset: isize) {
        let count = self
            .problems
            .as_ref()
            .map_or(0, |value| value.problems.len());
        if count > 0 {
            self.selected_problem =
                (self.selected_problem as isize + offset).rem_euclid(count as isize) as usize;
        }
    }

    pub async fn repair(
        client: &SharedClient,
        problem_id: String,
        action: String,
    ) -> Result<HealthRepairResultDto, CliError> {
        client
            .post_json(
                "/v1/health/repair",
                &HealthRepairRequestDto { problem_id, action },
            )
            .await
    }
}
