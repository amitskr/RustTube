//! A single running instance: a second launch (e.g. with a YouTube link on the
//! command line) hands its argument to the first and exits.
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

/// Turns a `rusttube:` (or legacy `streamtube:`) link back into the YouTube URL it wraps.
/// Accepts `rusttube://open?url=<encoded>`, `rusttube:https://…` and
/// `rusttube://www.youtube.com/watch?v=…`; anything else is returned unchanged.
pub fn normalize(arg: &str) -> String {
    let a = arg.trim().trim_matches('"');
    let Some(rest) = a
        .strip_prefix("rusttube:")
        .or_else(|| a.strip_prefix("RUSTTUBE:"))
        .or_else(|| a.strip_prefix("streamtube:"))
        .or_else(|| a.strip_prefix("STREAMTUBE:"))
    else {
        return a.to_string();
    };
    let rest = rest.trim_start_matches('/');
    if let Some(q) = rest.strip_prefix("open?").or_else(|| rest.strip_prefix("open/?")) {
        if let Some(u) = q.split('&').find_map(|kv| kv.strip_prefix("url=")) {
            return crate::api::http::urldecode(u);
        }
        if let Some(v) = q.split('&').find_map(|kv| kv.strip_prefix("v=")) {
            return format!("https://www.youtube.com/watch?v={v}");
        }
    }
    if rest.starts_with("http:") || rest.starts_with("https:") {
        // "https:/www…" when a browser collapsed the slashes
        let (scheme, tail) = rest.split_once(':').unwrap_or(("https", ""));
        return format!("{scheme}://{}", tail.trim_start_matches('/'));
    }
    format!("https://{rest}")
}

fn port_file() -> std::path::PathBuf {
    crate::store::data_dir().join("instance")
}

/// Hands `arg` to an already running instance. True when one took it (this process should exit).
pub fn forward_to_running(arg: &str) -> bool {
    let Ok(s) = std::fs::read_to_string(port_file()) else { return false };
    let mut parts = s.split_whitespace();
    let (Some(port), pid) = (parts.next().and_then(|p| p.parse::<u16>().ok()), parts.next().and_then(|p| p.parse::<u32>().ok())) else {
        return false;
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(400)) else { return false };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
    #[cfg(windows)]
    if let Some(pid) = pid {
        // let the running instance bring its window to the front
        #[link(name = "user32")]
        unsafe extern "system" {
            fn AllowSetForegroundWindow(pid: u32) -> i32;
        }
        unsafe { AllowSetForegroundWindow(pid) };
    }
    #[cfg(not(windows))]
    let _ = pid;
    if stream.write_all(format!("RUSTTUBE {}\n", arg.replace('\n', " ")).as_bytes()).is_err() {
        return false;
    }
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).is_ok() && reply.trim() == "ok"
}

/// Listens for links from later launches; each arrives on the returned channel.
pub fn listen() -> Option<futures::channel::mpsc::UnboundedReceiver<String>> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    std::fs::write(port_file(), format!("{port} {}", std::process::id())).ok()?;
    let (tx, rx) = futures::channel::mpsc::unbounded();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            let mut reader = BufReader::new(&stream);
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            // only our own launches speak this
            let trimmed = line.trim_end();
            let Some(arg) = trimmed.strip_prefix("RUSTTUBE ").or_else(|| trimmed.strip_prefix("STREAMTUBE ")) else {
                continue;
            };
            let _ = (&stream).write_all(b"ok\n");
            if tx.unbounded_send(arg.to_string()).is_err() {
                break;
            }
        }
    });
    Some(rx)
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn scheme_links() {
        assert_eq!(normalize("rusttube://open?url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3Dabc"), "https://www.youtube.com/watch?v=abc");
        assert_eq!(normalize("rusttube:https://youtu.be/abc"), "https://youtu.be/abc");
        assert_eq!(normalize("streamtube://open?url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3Dabc"), "https://www.youtube.com/watch?v=abc");
        assert_eq!(normalize("streamtube:https://youtu.be/abc"), "https://youtu.be/abc");
        assert_eq!(normalize("streamtube://https:/youtu.be/abc"), "https://youtu.be/abc");
        assert_eq!(normalize("streamtube://www.youtube.com/watch?v=abc"), "https://www.youtube.com/watch?v=abc");
        assert_eq!(normalize("https://youtu.be/abc"), "https://youtu.be/abc");
    }
}
