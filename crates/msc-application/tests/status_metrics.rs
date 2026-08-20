use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    ServerId,
};
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::metrics::{
    BoundedMetricHistory, ProcessMetricsProvider, ProcessResourceUsage,
};
use msc_infrastructure::process::{FakeProcessSupervisor, ProcessId, ProcessSpawnRequest};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

struct FakeRepository {
    server: ImportedJavaServer,
}

impl JavaServerRepository for FakeRepository {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        Ok((&self.server.id == id).then(|| self.server.clone()))
    }
}

#[derive(Default)]
struct FakeConsole {
    lines: Mutex<Vec<String>>,
}

impl ConsoleSink for FakeConsole {
    fn append_system_line(&self, _server_id: &ServerId, line: &str) {
        self.lines.lock().unwrap().push(line.to_string());
    }
}

struct FakeMetrics {
    pid: ProcessId,
    usage: ProcessResourceUsage,
}

impl ProcessMetricsProvider for FakeMetrics {
    fn process_usage(&self, pid: ProcessId) -> Option<ProcessResourceUsage> {
        (pid == self.pid).then_some(self.usage)
    }
}

fn temp_server_dir(case: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("msc2-status-metrics-{}-{case}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("world/region")).unwrap();
    fs::write(path.join("world/level.dat"), vec![0; 512 * 1024]).unwrap();
    fs::write(path.join("world/region/r.0.0.mca"), vec![0; 1536 * 1024]).unwrap();
    path
}

fn service<'deps>(
    repository: &'deps FakeRepository,
    process: &'deps FakeProcessSupervisor,
    console: &'deps FakeConsole,
    fs: &'deps FakeFileSystem,
) -> LifecycleService<'deps> {
    LifecycleService::new(repository, process, console, fs)
}

#[test]
fn status_metrics_snapshot_reports_active_paper_process_state() {
    let server_dir = temp_server_dir("active");
    let server = ImportedJavaServer::paper("paper-1", "Survival", &server_dir);
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let pid = service
        .start_active_server(
            ProcessSpawnRequest::new("/usr/bin/java", &server_dir).args([
                "-Xms1024M",
                "-Xmx2G",
                "-jar",
                "paper.jar",
            ]),
        )
        .unwrap();
    service
        .ingest_console_line("Done (1.234s)! For help, type \"help\"")
        .unwrap();
    service
        .ingest_console_line("camkage joined the game")
        .unwrap();
    service
        .ingest_console_line("TPS from last 1m, 5m, 15m: 19.8, 19.7, 19.6")
        .unwrap();

    let status = service.status_snapshot().unwrap();
    assert!(status.running);
    assert_eq!(status.active_server_id.as_deref(), Some("paper-1"));
    assert_eq!(status.pid, Some(i64::from(pid.raw())));
    assert_eq!(status.server_type.as_deref(), Some("paper"));

    let metrics = FakeMetrics {
        pid,
        usage: ProcessResourceUsage {
            cpu_percent: Some(37.5),
            ram_used_mb: Some(768.0),
        },
    };
    let snapshot = service
        .performance_snapshot(&metrics, "2026-08-02T00:00:00Z")
        .unwrap();
    assert_eq!(snapshot.ts, "2026-08-02T00:00:00Z");
    assert_eq!(snapshot.tps_1m, Some(19.8));
    assert_eq!(snapshot.players_online, Some(1));
    assert_eq!(snapshot.cpu_percent, Some(37.5));
    assert_eq!(snapshot.ram_used_mb, Some(768.0));
    assert_eq!(snapshot.ram_max_mb, Some(2048.0));
    assert_eq!(snapshot.world_size_mb, Some(2.0));
    assert_eq!(snapshot.server_type.as_deref(), Some("paper"));

    fs::remove_dir_all(server_dir).unwrap();
}

#[test]
fn status_metrics_stopped_snapshot_does_not_invent_process_metrics() {
    let server_dir = temp_server_dir("stopped");
    let server = ImportedJavaServer::paper("paper-1", "Survival", &server_dir);
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let metrics = FakeMetrics {
        pid: ProcessId::new(9999),
        usage: ProcessResourceUsage {
            cpu_percent: Some(99.0),
            ram_used_mb: Some(4096.0),
        },
    };

    let status = service.status_snapshot().unwrap();
    assert!(!status.running);
    assert_eq!(status.active_server_id.as_deref(), Some("paper-1"));
    assert_eq!(status.pid, None);

    let snapshot = service.performance_snapshot(&metrics, "ts").unwrap();
    assert_eq!(snapshot.tps_1m, None);
    assert_eq!(snapshot.players_online, Some(0));
    assert_eq!(snapshot.cpu_percent, None);
    assert_eq!(snapshot.ram_used_mb, None);
    assert_eq!(snapshot.world_size_mb, Some(2.0));

    fs::remove_dir_all(server_dir).unwrap();
}

#[test]
fn status_metrics_history_is_bounded() {
    let mut history = BoundedMetricHistory::new(3);
    history.push(1);
    history.push(2);
    history.push(3);
    history.push(4);

    assert_eq!(history.len(), 3);
    assert_eq!(
        history.samples().copied().collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
}
