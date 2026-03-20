//! Unix socket control interface for live parameter adjustment.
//!
//! Listens on `$XDG_RUNTIME_DIR/wl-walls.sock` (or `/tmp/` as
//! fallback) for line-based text commands:
//!
//!   get                 — dump all parameters as key=value lines
//!   set <param> <value> — update a parameter
//!   restart             — clear canvas + reset time (keep current params)
//!   randomize           — new random pendulum params + clear
//!   next-color          — cycle to next foreground color

use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use log::{info, warn};

pub struct ControlSocket {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl ControlSocket {
    pub fn bind() -> std::io::Result<Self> {
        let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        let socket_path = PathBuf::from(dir).join("wl-walls.sock");
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)?;
        listener.set_nonblocking(true)?;
        info!("Control socket: {}", socket_path.display());
        Ok(Self {
            listener,
            socket_path,
        })
    }

    /// Accept all pending connections, read one command from each.
    /// Returns (command_string, stream) pairs — caller writes the response
    /// to the stream and drops it.
    pub fn collect_pending(&self) -> Vec<(String, UnixStream)> {
        let mut pending = Vec::new();
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_millis(50)))
                        .ok();
                    let mut buf = [0u8; 4096];
                    match stream.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            let cmd = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                            pending.push((cmd, stream));
                        }
                        _ => {}
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    warn!("Control socket accept error: {}", e);
                    break;
                }
            }
        }
        pending
    }
}

impl Drop for ControlSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
