//! User-session companion; the service itself has no graphical dependencies.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

trait Connection: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Connection for T {}

pub(super) fn stop() -> Result<(), String> {
    use std::io::Write;
    let directory = super::agent_data_directory()?;
    let Ok(bytes) = std::fs::read(directory.join("agent-indicator-control.json")) else {
        return Ok(());
    };
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let port = value["port"]
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .ok_or("Invalid indicator control port")?;
    let token = value["token"]
        .as_str()
        .ok_or("Invalid indicator control token")?;
    if let Ok(mut stream) = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_secs(1),
    ) {
        stream
            .set_write_timeout(Some(std::time::Duration::from_secs(1)))
            .map_err(|error| error.to_string())?;
        stream
            .write_all(token.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.join("agent-indicator.lock"))
        .map_err(|error| error.to_string())?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if fs2::FileExt::try_lock_exclusive(&lock).is_ok() {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err("Agent indicator did not exit before package replacement".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn helmet() -> tauri::image::Image<'static> {
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..16 {
        for x in 0..16 {
            let outer = (1..14).contains(&y)
                && (1..15).contains(&x)
                && (y >= 2 || (4..12).contains(&x))
                && (y >= 4 || (2..14).contains(&x));
            let face = (y >= 8 && (6..10).contains(&x)) || (y >= 9 && (4..12).contains(&x));
            if outer && !face {
                for dy in 0..2 {
                    for dx in 0..2 {
                        let offset = ((y * 2 + dy) * 32 + x * 2 + dx) * 4;
                        pixels[offset..offset + 4].copy_from_slice(&[240, 240, 240, 255]);
                    }
                }
            }
        }
    }
    // A dark one-pixel edge keeps the white helmet legible on light taskbars.
    // macOS uses the alpha mask as a native template instead.
    #[cfg(not(target_os = "macos"))]
    {
        let original = pixels.clone();
        for y in 1..31 {
            for x in 1..31 {
                let offset = (y * 32 + x) * 4;
                if original[offset + 3] == 0
                    && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                        .iter()
                        .any(|&(nx, ny)| original[(ny * 32 + nx) * 4 + 3] != 0)
                {
                    pixels[offset..offset + 4].copy_from_slice(&[25, 25, 25, 255]);
                }
            }
        }
    }
    tauri::image::Image::new_owned(pixels, 32, 32)
}

async fn connect() -> Result<BufReader<Box<dyn Connection>>, String> {
    #[cfg(unix)]
    let stream: Box<dyn Connection> = {
        let directory = std::env::var_os("MSC2_DATA_DIR")
            .map(std::path::PathBuf::from)
            .map(Ok)
            .unwrap_or_else(super::agent_data_directory)?;
        let stream = tokio::net::UnixStream::connect(directory.join("local-cli.sock"))
            .await
            .map_err(|error| error.to_string())?;
        #[cfg(target_os = "macos")]
        let peer = msc_platform_macos::local_cli::peer_uid(&stream)?;
        #[cfg(target_os = "linux")]
        let peer = msc_platform_linux::local_cli::peer_uid(&stream)?;
        #[cfg(target_os = "macos")]
        let owner = msc_platform_macos::local_cli::service_uid();
        #[cfg(target_os = "linux")]
        let owner = msc_platform_linux::local_cli::service_uid();
        if peer != owner {
            return Err("Local agent belongs to another account".into());
        }
        Box::new(stream)
    };
    #[cfg(windows)]
    let stream: Box<dyn Connection> = {
        let mut stream = tokio::net::windows::named_pipe::ClientOptions::new()
            .open(msc_platform_windows::local_cli::PIPE_NAME)
            .map_err(|error| error.to_string())?;
        stream
            .write_all(b"{\"version\":1}\n")
            .await
            .map_err(|error| error.to_string())?;
        Box::new(stream)
    };
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .await
        .map_err(|error| error.to_string())?;
    let response: serde_json::Value =
        serde_json::from_str(&response).map_err(|error| error.to_string())?;
    if response["status"] != "ok" {
        return Err("Local agent refused indicator access".into());
    }
    reader
        .get_mut()
        .write_all(b"watch\n")
        .await
        .map_err(|error| error.to_string())?;
    let mut acknowledgement = String::new();
    reader
        .read_line(&mut acknowledgement)
        .await
        .map_err(|error| error.to_string())?;
    if acknowledgement != "running\n" {
        return Err("Agent does not support the indicator".into());
    }
    Ok(reader)
}

pub(super) fn run() {
    use fs2::FileExt;
    let Ok(directory) = super::agent_data_directory() else {
        return;
    };
    if !directory.exists() {
        let _ = super::indicator_startup::remove_registration();
        return;
    }
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let Ok(lock) = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("agent-indicator.lock"))
    else {
        return;
    };
    if lock.try_lock_exclusive().is_err() {
        return;
    }
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let status = MenuItem::with_id(app, "status", "Agent running", false, None::<&str>)?;
            let open = MenuItem::with_id(app, "open", "Open MSC 2", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit indicator", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status, &open, &quit])?;
            let tray = TrayIconBuilder::with_id("agent-running")
                .icon(helmet())
                .icon_as_template(true)
                .tooltip("MSC 2 agent running")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "open" => {
                        if let Ok(executable) = std::env::current_exe() {
                            let _ = std::process::Command::new(executable).spawn();
                        }
                    }
                    _ => {}
                })
                .build(app)?;
            tray.set_visible(false)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let Ok(listener) = tokio::net::TcpListener::bind("127.0.0.1:0").await else {
                    return;
                };
                let token = uuid::Uuid::new_v4().to_string();
                let Ok(directory) = super::agent_data_directory() else {
                    return;
                };
                let Ok(address) = listener.local_addr() else {
                    return;
                };
                let path = directory.join("agent-indicator-control.json");
                let bytes =
                    serde_json::to_vec(&serde_json::json!({"port":address.port(), "token":token}))
                        .unwrap_or_default();
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create(true).truncate(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                if let Ok(mut file) = options.open(&path) {
                    use std::io::Write;
                    if file.write_all(&bytes).is_err() {
                        return;
                    }
                } else {
                    return;
                }
                while let Ok((mut stream, _)) = listener.accept().await {
                    let mut supplied = vec![0u8; token.len()];
                    if matches!(
                        tokio::time::timeout(
                            std::time::Duration::from_secs(1),
                            stream.read_exact(&mut supplied)
                        )
                        .await,
                        Ok(Ok(_))
                    ) && supplied == token.as_bytes()
                    {
                        let _ = std::fs::remove_file(&path);
                        handle.exit(0);
                        break;
                    }
                }
            });
            tauri::async_runtime::spawn(async move {
                let mut delay = 1;
                loop {
                    if super::agent_data_directory().is_ok_and(|path| !path.exists()) {
                        let _ = super::indicator_startup::remove_registration();
                        handle_exit();
                        return;
                    }
                    let connection =
                        tokio::time::timeout(std::time::Duration::from_secs(3), connect()).await;
                    if let Ok(Ok(mut connection)) = connection {
                        delay = 1;
                        let _ = tray.set_visible(true);
                        let mut byte = [0u8; 1];
                        let _ = connection.read(&mut byte).await;
                        let _ = tray.set_visible(false);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    delay = (delay * 2).min(30);
                }
            });
            Ok(())
        })
        .build(context)
        .expect("could not start the agent indicator");
    app.run(|_, event| {
        if let tauri::RunEvent::ExitRequested {
            api, code: None, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}

fn handle_exit() {
    // No server action is associated with ending this graphical companion.
    std::process::exit(0);
}
