//! Optional lifecycle subscription following the installing-user CLI exchange.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(super) async fn wait<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S) -> Result<(), String> {
    // Existing CLI clients close after receiving their credential. Only the
    // indicator opts into keeping this authenticated local connection open.
    let mut request = [0u8; 6];
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read_exact(&mut request),
    )
    .await
    {
        Ok(Ok(_)) if &request == b"watch\n" => {}
        _ => return Ok(()),
    }
    stream
        .write_all(b"running\n")
        .await
        .map_err(|error| format!("acknowledging local agent watch: {error}"))?;
    let mut byte = [0u8; 1];
    // No timer or heartbeat: process exit closes the connection in the OS.
    let _ = stream.read(&mut byte).await;
    Ok(())
}
