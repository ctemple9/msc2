#![cfg(target_os = "linux")]

use msc_infrastructure::service::{
    ServiceError, ServiceInstallRequest, ServiceManager, ServiceState,
};
use msc_platform_linux::service::{
    self, AuthorizationRunner, LinuxSystemdServiceManager, Systemctl,
};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "msc-desktop-service-elevation-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated temporary root");
        Self(fs::canonicalize(path).expect("resolve isolated temporary root"))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Default)]
struct FakeSystemctl(Arc<Mutex<Vec<String>>>);

impl FakeSystemctl {
    fn record(&self, command: impl Into<String>) {
        self.0
            .lock()
            .expect("fake systemctl lock")
            .push(command.into());
    }

    fn calls(&self) -> Vec<String> {
        self.0.lock().expect("fake systemctl lock").clone()
    }
}

impl Systemctl for FakeSystemctl {
    fn daemon_reload(&self) -> Result<(), ServiceError> {
        self.record("daemon-reload");
        Ok(())
    }

    fn enable(&self, unit_name: &str) -> Result<(), ServiceError> {
        self.record(format!("enable {unit_name}"));
        Ok(())
    }

    fn disable(&self, unit_name: &str) -> Result<(), ServiceError> {
        self.record(format!("disable {unit_name}"));
        Ok(())
    }

    fn start(&self, unit_name: &str) -> Result<(), ServiceError> {
        self.record(format!("start {unit_name}"));
        Ok(())
    }

    fn stop(&self, unit_name: &str) -> Result<(), ServiceError> {
        self.record(format!("stop {unit_name}"));
        Ok(())
    }

    fn show(&self, unit_name: &str) -> Result<String, ServiceError> {
        self.record(format!("show {unit_name}"));
        Ok("ActiveState=active\nMainPID=4321\n".to_string())
    }
}

struct FakeAuthorizationRunner {
    request: ServiceInstallRequest,
    uid: u32,
    user: String,
    home: PathBuf,
    manager: Arc<dyn ServiceManager>,
    calls: Mutex<Vec<(PathBuf, Vec<OsString>)>>,
}

impl AuthorizationRunner for FakeAuthorizationRunner {
    fn authorize(&self, helper: &Path, args: &[OsString]) -> Result<Output, ServiceError> {
        self.calls
            .lock()
            .expect("fake authorization lock")
            .push((helper.to_path_buf(), args.to_vec()));
        match args
            .get(1)
            .map(|value| value.to_string_lossy().to_string())
            .as_deref()
        {
            Some("install") => service::apply_desktop_service_helper_install(
                &self.request,
                self.uid,
                &self.user,
                &self.home,
                self.manager.as_ref(),
            )?,
            Some("uninstall") => {
                service::apply_desktop_service_helper_uninstall(&self.user, self.manager.as_ref())?
            }
            _ => {
                return Err(ServiceError::Platform(
                    "fake authorizer received an unexpected helper command".to_string(),
                ));
            }
        }
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

#[test]
fn desktop_install_repair_and_uninstall_use_only_the_fake_privileged_boundary() {
    let temporary = TestDirectory::new();
    let uid = unsafe { libc::getuid() };
    let user = std::env::var("USER").expect("test user name is available");
    let home = temporary.0.join("home");
    let data = home.join(".local/share/msc2");
    let logs = data.join("logs");
    let secrets = data.join("secrets");
    let builds = data.join("agent/builds");
    let staged = builds.join("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    let unit_root = temporary.0.join("systemd");
    let helper = temporary.0.join("msc");

    for directory in [&home, &logs, &secrets, &staged] {
        fs::create_dir_all(directory).expect("create fixture directory");
        fs::set_permissions(directory, fs::Permissions::from_mode(0o755))
            .expect("make fixture directory owner-only writable");
    }
    fs::write(&helper, b"fake helper executable").expect("write fake helper marker");
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o755))
        .expect("make fake helper executable");
    let binary = staged.join("msc");
    fs::write(&binary, b"fake agent executable").expect("write staged agent marker");
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))
        .expect("make staged agent executable");

    let request = ServiceInstallRequest::new(
        service::DESKTOP_AGENT_SERVICE_NAME,
        &binary,
        &data,
        logs.join("agent.log"),
        48001,
    )
    .args(["serve", "--bind", "127.0.0.1:48001"])
    .env("HOME", home.display().to_string())
    .env("MSC2_DATA_DIR", data.display().to_string())
    .env("MSC2_MACOS_SECRET_STORE_DIR", secrets.display().to_string())
    .env(
        "MSC2_LOCAL_BOOTSTRAP_SOCKET",
        data.join("local-bootstrap.sock").display().to_string(),
    )
    .run_user(user.clone());
    let fake_systemctl = FakeSystemctl::default();
    let helper_manager = Arc::new(LinuxSystemdServiceManager::with_systemctl(
        &unit_root,
        fake_systemctl.clone(),
    ));
    let status_manager =
        LinuxSystemdServiceManager::with_systemctl(&unit_root, fake_systemctl.clone());
    let runner = FakeAuthorizationRunner {
        request: request.clone(),
        uid,
        user: user.clone(),
        home: home.clone(),
        manager: helper_manager,
        calls: Mutex::new(Vec::new()),
    };

    for _ in 0..2 {
        let report = service::install_desktop_service_with_runner_for_identity(
            request.clone(),
            &helper,
            &runner,
            uid,
            &user,
            &home,
            &status_manager,
        )
        .expect("desktop install and repair use fake authorization");
        assert_eq!(report.state, ServiceState::Running);
        assert_eq!(report.pid, Some(4321));
        let unit = fs::read_to_string(
            unit_root.join(format!("{}.service", service::DESKTOP_AGENT_SERVICE_NAME)),
        )
        .expect("read temporary MSC unit");
        assert!(unit.contains(&format!("User={user}\nGroup={user}\n")));
        assert!(unit.contains(&format!("WorkingDirectory={}\n", data.display())));
        assert_eq!(
            fs::metadata(&data)
                .expect("inspect MSC data directory")
                .uid(),
            uid
        );
        assert!(!unit_root.starts_with("/etc/systemd/system"));
    }

    let uninstalled = service::uninstall_desktop_service_with_runner(&helper, &runner)
        .expect("desktop uninstall uses fake authorization");
    assert_eq!(uninstalled.state, ServiceState::NotInstalled);
    assert!(
        !unit_root
            .join(format!("{}.service", service::DESKTOP_AGENT_SERVICE_NAME))
            .exists()
    );
    let authorization_calls = runner.calls.lock().expect("fake authorization lock");
    assert_eq!(authorization_calls.len(), 3);
    assert!(
        authorization_calls
            .iter()
            .all(|(program, _)| program == &helper)
    );
    assert_eq!(
        authorization_calls[0].1[0].to_string_lossy(),
        "desktop-service-helper"
    );
    assert_eq!(authorization_calls[0].1[1].to_string_lossy(), "install");
    assert_eq!(authorization_calls[2].1[1].to_string_lossy(), "uninstall");
    let calls = fake_systemctl.calls();
    assert!(
        calls
            .iter()
            .all(|call| call.contains("com.ctemple.msc2.agent.service") || call == "daemon-reload")
    );
    assert!(
        calls
            .iter()
            .any(|call| call == "start com.ctemple.msc2.agent.service")
    );
    assert!(
        calls
            .iter()
            .any(|call| call == "stop com.ctemple.msc2.agent.service")
    );
}
