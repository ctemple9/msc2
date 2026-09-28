//! The production `msc.exe` entry point used by the installed Windows service.

use std::ffi::OsString;
use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::oneshot;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{define_windows_service, service_dispatcher};

use crate::run_service_with_shutdown;

static CONFIG: OnceLock<(String, SocketAddr)> = OnceLock::new();

define_windows_service!(service_main_ffi, service_main);

pub fn dispatch(service_name: String, bind: SocketAddr) -> Result<(), String> {
    CONFIG
        .set((service_name.clone(), bind))
        .map_err(|_| "Windows service configuration was already set".to_string())?;
    service_dispatcher::start(service_name, service_main_ffi).map_err(|error| error.to_string())
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run() {
        eprintln!("msc Windows service failed: {error}");
    }
}

fn run() -> Result<(), String> {
    let (service_name, bind) = CONFIG
        .get()
        .ok_or("Windows service configuration is missing")?;
    let (stop_tx, stop_rx) = oneshot::channel();
    let stop_tx = Mutex::new(Some(stop_tx));
    let handler = move |control| match control {
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        ServiceControl::Stop | ServiceControl::Shutdown => {
            if let Some(sender) = stop_tx.lock().expect("stop lock poisoned").take() {
                let _ = sender.send(());
            }
            ServiceControlHandlerResult::NoError
        }
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let status_handle = service_control_handler::register(service_name, handler)
        .map_err(|error| error.to_string())?;
    status_handle
        .set_service_status(status(ServiceState::StartPending))
        .map_err(|error| error.to_string())?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let result = runtime.block_on(run_service_with_shutdown(
        *bind,
        async move {
            let _ = stop_rx.await;
        },
        || {
            status_handle
                .set_service_status(status(ServiceState::Running))
                .map_err(|error| crate::cli::CliError::internal(error.to_string()))
        },
    ));
    status_handle
        .set_service_status(status(ServiceState::StopPending))
        .map_err(|error| error.to_string())?;
    let mut final_status = status(ServiceState::Stopped);
    if result.is_err() {
        final_status.exit_code = ServiceExitCode::Win32(1);
    }
    status_handle
        .set_service_status(final_status)
        .map_err(|error| error.to_string())?;
    result.map_err(|error| error.to_string())
}

fn status(state: ServiceState) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: if state == ServiceState::Running {
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
        } else {
            ServiceControlAccept::empty()
        },
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: if matches!(
            state,
            ServiceState::StartPending | ServiceState::StopPending
        ) {
            1
        } else {
            0
        },
        wait_hint: if matches!(
            state,
            ServiceState::StartPending | ServiceState::StopPending
        ) {
            Duration::from_secs(30)
        } else {
            Duration::ZERO
        },
        process_id: None,
    }
}
