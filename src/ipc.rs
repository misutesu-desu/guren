//! Lets a new launch hand its photo to an instance that is already running in
//! the background, over a named pipe. The new process exits right away, the
//! running one shows its window, so the photo appears without a GPU startup.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::windows::io::FromRawHandle;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

use eframe::egui;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Foundation::{ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GetLastError};
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};

const ACCEPTED: u8 = b'1';
const REFUSED: u8 = b'0';

/// Pipe names are machine wide, the user name keeps two signed-in users apart.
fn pipe_name() -> String {
    let user = std::env::var("USERNAME").unwrap_or_default();
    format!(r"\\.\pipe\guren-{user}")
}

/// Sends `path` to a running instance. Returns false when there is none or it
/// is not keeping itself in the background, then this process starts normally.
pub fn hand_off(path: Option<&Path>) -> bool {
    hand_off_to(&pipe_name(), path)
}

fn hand_off_to(name: &str, path: Option<&Path>) -> bool {
    let Some(mut pipe) = connect(name) else {
        return false;
    };
    // The running instance may only bring its window to the front if the
    // process the user just launched allows it.
    unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    let line = path
        .and_then(|p| std::path::absolute(p).ok())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut answer = [0u8];
    pipe.write_all(format!("{line}\n").as_bytes()).is_ok()
        && pipe.read_exact(&mut answer).is_ok()
        && answer[0] == ACCEPTED
}

fn connect(name: &str) -> Option<File> {
    for _ in 0..20 {
        match OpenOptions::new().read(true).write(true).open(name) {
            Ok(f) => return Some(f),
            // Another launch is talking to the server right now.
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
    None
}

/// The listening side. Started once, then only switched on and off, because a
/// pipe that stops existing would make launches wait for nothing.
pub struct Server {
    enabled: Arc<AtomicBool>,
}

impl Server {
    /// `None` when another process already owns the pipe.
    pub fn start(ctx: &egui::Context, inbox: Sender<Option<PathBuf>>) -> Option<Self> {
        Self::start_at(&pipe_name(), ctx, inbox)
    }

    fn start_at(name: &str, ctx: &egui::Context, inbox: Sender<Option<PathBuf>>) -> Option<Self> {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let first = create_pipe(&name, true)?;
        let enabled = Arc::new(AtomicBool::new(true));
        let flag = enabled.clone();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("ipc".into())
            .spawn(move || {
                // The next instance exists before the current one closes, so a
                // launch never finds the name missing and starts a second copy.
                let mut current = first;
                while let Some(next) = create_pipe(&name, false) {
                    serve(current, &flag, &inbox, &ctx);
                    current = next;
                }
            })
            .ok()?;
        Some(Self { enabled })
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
    }
}

/// `first` refuses to create the pipe when another process already owns the name.
fn create_pipe(name: &[u16], first: bool) -> Option<File> {
    let first_flag = if first {
        FILE_FLAG_FIRST_PIPE_INSTANCE
    } else {
        0
    };
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | first_flag,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            4096,
            4096,
            0,
            std::ptr::null(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    Some(unsafe { File::from_raw_handle(handle) })
}

fn serve(file: File, enabled: &AtomicBool, inbox: &Sender<Option<PathBuf>>, ctx: &egui::Context) {
    use std::os::windows::io::AsRawHandle;
    let connected = unsafe { ConnectNamedPipe(file.as_raw_handle(), std::ptr::null_mut()) } != 0
        || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
    if !connected {
        return;
    }

    let mut reader = BufReader::new(&file);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let on = enabled.load(Ordering::Relaxed);
    if on {
        let line = line.trim_end_matches(['\r', '\n']);
        let path = (!line.is_empty()).then(|| PathBuf::from(line));
        if inbox.send(path).is_err() {
            return;
        }
        ctx.request_repaint();
    }
    let mut out = &file;
    let _ = out.write_all(&[if on { ACCEPTED } else { REFUSED }]);
    // Closing the handle throws away unread data. Flushing waits until the
    // client has read the answer. A client that already left needs nothing more.
    let _ = file.sync_all();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn test_pipe(tag: &str) -> String {
        format!(r"\\.\pipe\guren-test-{tag}-{}", std::process::id())
    }

    #[test]
    fn path_reaches_the_running_instance() {
        let name = test_pipe("on");
        let (tx, rx) = channel();
        let _server = Server::start_at(&name, &egui::Context::default(), tx).unwrap();
        assert!(hand_off_to(&name, Some(Path::new(r"C:\photos\a.jpg"))));
        assert_eq!(rx.recv().unwrap(), Some(PathBuf::from(r"C:\photos\a.jpg")));
        assert!(hand_off_to(&name, None));
        assert_eq!(rx.recv().unwrap(), None);
    }

    #[test]
    fn disabled_server_lets_the_launch_start_normally() {
        let name = test_pipe("off");
        let (tx, rx) = channel();
        let server = Server::start_at(&name, &egui::Context::default(), tx).unwrap();
        server.set_enabled(false);
        assert!(!hand_off_to(&name, Some(Path::new(r"C:\a.jpg"))));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn no_server_means_no_hand_off() {
        assert!(!hand_off_to(&test_pipe("none"), None));
    }

    #[test]
    fn second_server_on_the_same_name_is_refused() {
        let name = test_pipe("twice");
        let ctx = egui::Context::default();
        let _first = Server::start_at(&name, &ctx, channel().0).unwrap();
        assert!(Server::start_at(&name, &ctx, channel().0).is_none());
    }
}
