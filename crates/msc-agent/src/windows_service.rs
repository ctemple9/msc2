//! The production `msc.exe` entry point used by the installed Windows service.

use std::ffi::OsString;
use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::mpsc;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{define_windows_service, service_dispatcher};

use crate::run_service_with_shutdown;

static CONFIG: OnceLock<(String, SocketAddr)> = OnceLock::new();
static LIFECYCLE: OnceLock<crate::routes::worlds::WorldsRoutesState> = OnceLock::new();
static MAINTENANCE: Mutex<Option<msc_domain::operation::OperationId>> = Mutex::new(None);

pub(crate) fn register_lifecycle(state: crate::routes::worlds::WorldsRoutesState) {
    let _ = LIFECYCLE.set(state);
}

pub(crate) fn finish_maintenance() {
    if let (Some(state), Some(id)) = (LIFECYCLE.get(), MAINTENANCE.lock().unwrap().take()) {
        state.lifecycle.finish_service_maintenance(&id);
    }
}

pub(crate) fn maintenance_requested() -> bool {
    MAINTENANCE.lock().unwrap().is_some()
}

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
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel();
    let handler = move |control| match control {
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_tx.send(false);
            ServiceControlHandlerResult::NoError
        }
        // Only callers with SERVICE_USER_DEFINED_CONTROL can request this local
        // maintenance handshake. It exposes no remote API or arbitrary command.
        ServiceControl::UserEvent(code) if code.to_raw() == 128 => {
            let _ = stop_tx.send(true);
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
            while let Some(maintenance) = stop_rx.recv().await {
                if !maintenance {
                    break;
                }
                let Some(lifecycle) = LIFECYCLE.get().cloned() else {
                    continue;
                };
                lifecycle.begin_map_shutdown();
                let worker = lifecycle.clone();
                match tokio::task::spawn_blocking(move || {
                    worker.lifecycle.prepare_service_maintenance()
                })
                .await
                {
                    Ok(Ok(id)) => {
                        *MAINTENANCE.lock().unwrap() = Some(id);
                        let _ = status_handle.set_service_status(status(ServiceState::StopPending));
                        break;
                    }
                    Ok(Err(error)) => {
                        lifecycle.cancel_map_shutdown();
                        eprintln!("Local package maintenance refused: {error}");
                    }
                    Err(error) => {
                        lifecycle.cancel_map_shutdown();
                        eprintln!("Local package maintenance worker failed: {error}");
                    }
                }
            }
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
