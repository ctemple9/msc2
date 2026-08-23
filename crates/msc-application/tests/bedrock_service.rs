use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeCapabilities,
    BedrockRuntimeError, BedrockRuntimeEvent, BedrockRuntimeState, BedrockStartRequest,
    BedrockTerminationReason,
};
use msc_application::bedrock_service::{
    BEDROCK_CONSOLE_HISTORY_CAPACITY, BedrockServerInfo, BedrockService, BedrockServiceEvent,
};
use msc_application::operations::LifecycleOperations;
use msc_domain::operation::OperationState;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::metrics::{ProcessMetricsProvider, ProcessResourceUsage};
use msc_infrastructure::operation_journal::ReconciliationRecord;
use msc_infrastructure::process::ProcessId;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

const SERVER_DIR: &str = "/servers/bedrock/survival";
const OPERATIONS_DIR: &str = "/agent/operations";

struct FakeRuntime {
    state: BedrockRuntimeState,
    events: VecDeque<BedrockRuntimeEvent>,
    commands: Vec<String>,
    process_id: Option<ProcessId>,
    query_ready_after: Option<usize>,
    query_count: usize,
}

impl FakeRuntime {
    fn new() -> Self {
        Self {
            state: BedrockRuntimeState::New,
            events: VecDeque::new(),
            commands: Vec::new(),
            process_id: Some(ProcessId::new(77)),
            query_ready_after: None,
            query_count: 0,
        }
    }

    fn emit(&mut self, event: BedrockRuntimeEvent) {
        self.events.push_back(event);
    }
}

impl BedrockRuntime for FakeRuntime {
    fn capabilities(&self) -> &BedrockRuntimeCapabilities {
        static CAPABILITIES: std::sync::OnceLock<BedrockRuntimeCapabilities> =
            std::sync::OnceLock::new();
        CAPABILITIES
            .get_or_init(|| BedrockRuntimeCapabilities::supported(BedrockRuntimeBackend::Native))
    }

    fn state(&self) -> BedrockRuntimeState {
        self.state
    }

    fn process_id(&self) -> Option<ProcessId> {
        self.process_id
    }

    fn provision(&mut self, _request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError> {
        assert!(matches!(
            self.state,
            BedrockRuntimeState::New | BedrockRuntimeState::Stopped
        ));
        self.state = BedrockRuntimeState::Provisioned;
        Ok(())
    }

    fn start(&mut self, _request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        assert_eq!(self.state, BedrockRuntimeState::Provisioned);
        self.state = BedrockRuntimeState::Starting;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn force_stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError> {
        self.commands.push(command.to_owned());
        if command == "save query" {
            self.query_count += 1;
            if self.query_ready_after == Some(self.query_count) {
                self.events.push_back(BedrockRuntimeEvent::ConsoleLine(
                    "Data saved. Files are now ready to be copied.".into(),
                ));
            }
        }
        Ok(())
    }

    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        if let Some(BedrockRuntimeEvent::Terminated { .. }) = self.events.front() {
            self.state = BedrockRuntimeState::Stopped;
        }
        Ok(self.events.pop_front())
    }
}

struct FakeMetrics;

impl ProcessMetricsProvider for FakeMetrics {
    fn process_usage(&self, _pid: ProcessId) -> Option<ProcessResourceUsage> {
        Some(ProcessResourceUsage {
            cpu_percent: Some(12.5),
            ram_used_mb: Some(512.0),
        })
    }
}

fn service<'a>(
    runtime: FakeRuntime,
    fs: &'a FakeFileSystem,
    metrics: &'a FakeMetrics,
    operations: &'a LifecycleOperations<'a>,
) -> BedrockService<'a, 'a, FakeRuntime> {
    BedrockService::new(
        runtime,
        fs,
        metrics,
        operations,
        BedrockServerInfo {
            id: "bedrock-1".into(),
            name: "Survival".into(),
            directory: PathBuf::from(SERVER_DIR),
            version: "1.21.80.3".into(),
            memory_gb: 2,
            bedrock_port: 19132,
        },
    )
}

fn base_fs() -> FakeFileSystem {
    FakeFileSystem::new()
        .with_dir(OPERATIONS_DIR)
        .with_dir(SERVER_DIR)
        .with_file(
            format!("{SERVER_DIR}/allowlist.json"),
            br#"[{"name":"Alex","xuid":null}]"#.to_vec(),
            false,
        )
}

#[test]
fn lifecycle_logs_players_metrics_and_journal_state() {
    let fs = base_fs();
    let metrics = FakeMetrics;
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let mut service = service(FakeRuntime::new(), &fs, &metrics, &operations);

    service.start("2026-08-23T12:00:00Z").unwrap();
    assert_eq!(
        service.state(),
        msc_application::lifecycle::LifecycleState::Starting
    );
    assert_eq!(
        service.operation_snapshot().unwrap().unwrap().state,
        OperationState::Running
    );

    service.runtime_mut().emit(BedrockRuntimeEvent::Ready {
        address: None,
        port: 19132,
    });
    service.runtime_mut().emit(BedrockRuntimeEvent::ConsoleLine(
        "Player connected: Alex, xuid: 2535416361514257".into(),
    ));
    service.poll().unwrap();

    assert_eq!(
        service.state(),
        msc_application::lifecycle::LifecycleState::Running
    );
    assert_eq!(service.online_players()[0].name, "Alex");
    assert_eq!(
        service.online_players()[0].xuid.as_deref(),
        Some("2535416361514257")
    );
    assert_eq!(
        service.operation_snapshot().unwrap().unwrap().state,
        OperationState::Succeeded
    );
    assert_eq!(
        String::from_utf8(
            fs.read(Path::new(&format!("{SERVER_DIR}/allowlist.json")))
                .unwrap()
        )
        .unwrap(),
        "[\n  {\n    \"name\": \"Alex\",\n    \"xuid\": \"2535416361514257\",\n    \"ignores_player_limit\": false\n  }\n]"
    );

    let snapshot = service.performance_snapshot("ts");
    assert_eq!(snapshot.cpu_percent, Some(12.5));
    assert_eq!(snapshot.ram_used_mb, Some(512.0));
    assert_eq!(snapshot.players_online, 1);
    assert_eq!(snapshot.metric_history_len, 1);

    service.command("/say hello").unwrap();
    assert_eq!(service.runtime().commands, vec!["/say hello"]);

    service.stop().unwrap();
    service.runtime_mut().emit(BedrockRuntimeEvent::Terminated {
        reason: BedrockTerminationReason::Clean,
    });
    let events = service.poll().unwrap();
    assert!(events.contains(&BedrockServiceEvent::CleanStop));
    assert_eq!(
        service.state(),
        msc_application::lifecycle::LifecycleState::Stopped
    );
    assert_eq!(
        service.operation_snapshot().unwrap().unwrap().state,
        OperationState::Succeeded
    );
    assert!(
        String::from_utf8(
            fs.read(Path::new(&format!("{SERVER_DIR}/logs/latest.log")))
                .unwrap()
        )
        .unwrap()
        .contains("MSC Bedrock console log")
    );
}

#[test]
fn crash_is_distinct_and_can_be_restarted() {
    let fs = base_fs();
    let metrics = FakeMetrics;
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let mut service = service(FakeRuntime::new(), &fs, &metrics, &operations);
    service.start("2026-08-23T12:00:00Z").unwrap();
    service.runtime_mut().emit(BedrockRuntimeEvent::Ready {
        address: None,
        port: 19132,
    });
    service.poll().unwrap();
    service.runtime_mut().emit(BedrockRuntimeEvent::Terminated {
        reason: BedrockTerminationReason::GuestError("exit code 1".into()),
    });
    assert!(
        service
            .poll()
            .unwrap()
            .contains(&BedrockServiceEvent::Crash("exit code 1".into()))
    );
    assert_eq!(
        service.state(),
        msc_application::lifecycle::LifecycleState::Crashed
    );
    assert_eq!(
        service.operation_snapshot().unwrap().unwrap().state,
        OperationState::Failed
    );

    service.restart_after_crash("2026-08-23T12:01:00Z").unwrap();
    assert_eq!(
        service.state(),
        msc_application::lifecycle::LifecycleState::Starting
    );
}

#[test]
fn save_hold_query_timeout_is_best_effort_and_resume_runs() {
    let fs = base_fs();
    let metrics = FakeMetrics;
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let mut runtime = FakeRuntime::new();
    runtime.query_ready_after = Some(3);
    let mut service = service(runtime, &fs, &metrics, &operations);
    service.start("2026-08-23T12:00:00Z").unwrap();
    service.runtime_mut().emit(BedrockRuntimeEvent::Ready {
        address: None,
        port: 19132,
    });
    service.poll().unwrap();

    let (value, pause) = service.run_with_save_hold(10, || "backup-created").unwrap();
    assert_eq!(value, "backup-created");
    assert!(pause.saves_held);
    assert!(pause.ready_to_copy);
    assert_eq!(pause.query_attempts, 3);
    assert_eq!(
        service.runtime().commands,
        vec![
            "save hold",
            "save query",
            "save query",
            "save query",
            "save resume"
        ]
    );
}

#[test]
fn console_history_and_metric_history_are_bounded() {
    let fs = base_fs();
    let metrics = FakeMetrics;
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let mut service = service(FakeRuntime::new(), &fs, &metrics, &operations);
    service.start("2026-08-23T12:00:00Z").unwrap();
    service.runtime_mut().emit(BedrockRuntimeEvent::Ready {
        address: None,
        port: 19132,
    });
    service.poll().unwrap();
    for index in 0..(BEDROCK_CONSOLE_HISTORY_CAPACITY + 25) {
        service.ingest_console_line(&format!("line {index}"));
    }
    for _ in 0..100 {
        service.performance_snapshot("ts");
    }
    assert_eq!(
        service.console_tail(usize::MAX).len(),
        BEDROCK_CONSOLE_HISTORY_CAPACITY
    );
    assert_eq!(service.metric_history().count(), 60);
    assert_eq!(
        service.console_tail(1),
        vec![format!("line {}", BEDROCK_CONSOLE_HISTORY_CAPACITY + 24)]
    );
}

#[test]
fn startup_reconciliation_is_delegated_to_operation_journal() {
    let fs = base_fs();
    let metrics = FakeMetrics;
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let mut service = service(FakeRuntime::new(), &fs, &metrics, &operations);
    service.start("2026-08-23T12:00:00Z").unwrap();
    let restarted = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let records: Vec<ReconciliationRecord> = restarted.reconcile_on_startup().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].to, OperationState::Failed);
}
