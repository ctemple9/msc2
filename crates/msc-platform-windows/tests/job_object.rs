#![cfg(target_os = "windows")]

use msc_infrastructure::process::{
    OutputStream, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};
use msc_platform_windows::process::WindowsJavaProcessSupervisor;
use std::time::{Duration, Instant};

fn powershell(script: &str) -> ProcessSpawnRequest {
    ProcessSpawnRequest::new("powershell.exe", std::env::temp_dir()).args([
        "-NoProfile",
        "-Command",
        script,
    ])
}

/// 30s, not 10s: P7.29's CI runs found real `powershell.exe` spawns here
/// occasionally missed a 10s budget under heavy concurrent nextest load
/// on GitHub's hosted `windows-latest` runners -- a real scheduling/
/// startup-latency false failure under load, not a hang.
fn drain_until_exit(
    supervisor: &WindowsJavaProcessSupervisor,
    pid: ProcessId,
) -> Vec<ProcessEvent> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut events = Vec::new();
    while Instant::now() < deadline {
        events.extend(supervisor.drain_events(pid).unwrap());
        if events
            .iter()
            .any(|event| matches!(event, ProcessEvent::Exited(_)))
        {
            return events;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for process exit; events: {events:?}");
}

fn stdout_text(events: &[ProcessEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            ProcessEvent::Output {
                stream: OutputStream::Stdout,
                bytes,
            } => Some(String::from_utf8_lossy(bytes)),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn process_supervisor_real_streams_stdin_stdout_stderr_and_exit_code() {
    let supervisor = WindowsJavaProcessSupervisor::new();
    let pid = supervisor
        .spawn(powershell(
            r#"[Console]::Out.WriteLine('out'); [Console]::Error.WriteLine('err'); $line = [Console]::In.ReadLine(); [Console]::Out.WriteLine("cmd:$line"); exit 7"#,
        ))
        .unwrap();

    supervisor.write_stdin(pid, b"say hello\n").unwrap();
    let events = drain_until_exit(&supervisor, pid);

    assert!(stdout_text(&events).contains("out"));
    assert!(stdout_text(&events).contains("cmd:say hello"));
    assert!(events.iter().any(|event| matches!(
        event,
        ProcessEvent::Output {
            stream: OutputStream::Stderr,
            bytes
        } if String::from_utf8_lossy(bytes).contains("err")
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ProcessEvent::Exited(status) if status.code == Some(7)))
    );
}

#[test]
fn process_supervisor_real_graceful_stop_writes_stop_newline() {
    let supervisor = WindowsJavaProcessSupervisor::new();
    let pid = supervisor
        .spawn(powershell(
            r#"$line = [Console]::In.ReadLine(); if ($line -eq 'stop') { exit 0 } else { exit 2 }"#,
        ))
        .unwrap();

    supervisor.request_graceful_stop(pid).unwrap();
    let events = drain_until_exit(&supervisor, pid);

    assert!(
        events
            .iter()
            .any(|event| matches!(event, ProcessEvent::Exited(status) if status.success()))
    );
}

#[test]
fn process_supervisor_real_job_object_terminates_child_process_tree() {
    let supervisor = WindowsJavaProcessSupervisor::new();
    let pid = supervisor
        .spawn(powershell(
            r#"$p = Start-Process -FilePath powershell.exe -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru; [Console]::Out.WriteLine("child:$($p.Id)"); Start-Sleep -Seconds 30"#,
        ))
        .unwrap();

    let child_pid = wait_for_child_pid(&supervisor, pid);
    supervisor.force_terminate(pid).unwrap();
    let events = drain_until_exit(&supervisor, pid);

    assert!(
        events
            .iter()
            .any(|event| matches!(event, ProcessEvent::Exited(_)))
    );
    assert_child_process_is_gone(child_pid);
}

fn wait_for_child_pid(supervisor: &WindowsJavaProcessSupervisor, pid: ProcessId) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut output = String::new();
    while Instant::now() < deadline {
        output.push_str(&stdout_text(&supervisor.drain_events(pid).unwrap()));
        if let Some(raw) = output
            .lines()
            .find_map(|line| line.strip_prefix("child:"))
            .and_then(|raw| raw.trim().parse::<u32>().ok())
        {
            return raw;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for child pid; output: {output}");
}

fn assert_child_process_is_gone(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let status = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 1 }} else {{ exit 0 }}"
                ),
            ])
            .status()
            .expect("checking child process status");
        if status.success() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("child process {pid} was still running after Job Object termination");
}
