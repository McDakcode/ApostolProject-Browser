// Made by MrDuck && Ox-Alpha
//! Local filtering HTTP proxy — the network-layer enforcement point for the
//! privacy engine (roadmap item #2: "privacy applied to real traffic").
//!
//! WebView2 routes all tab traffic through `--proxy-server=http://127.0.0.1:P`
//! (see `liveprivacy::init_browser_args`). For every request the proxy asks
//! the live `PrivacySnap` whether the target host is a tracker / ad /
//! malicious domain and either blocks it or pipes it through untouched.
//!
//! Blocking is purely host-based (CONNECT authority / Host header) — no TLS
//! interception, no certificates, nothing leaves the machine. HTTPS tunnels
//! are allowed or denied whole; plain HTTP requests are forwarded verbatim.
//!
//! Threading: blocking std::TcpListener + one thread per connection. A
//! desktop browser keeps dozens of connections alive at worst — cheap.

use crate::liveprivacy::LiveFilter;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

const PORT_RANGE: std::ops::RangeInclusive<u16> = 47790..=47799;
const HEAD_CAP: usize = 64 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Bind the loopback listener and start the accept thread.
/// Returns the chosen port (None = every candidate port was busy).
pub fn spawn(filter: Arc<LiveFilter>) -> Option<u16> {
    let listener = PORT_RANGE
        .clone()
        .filter_map(|p| TcpListener::bind(("127.0.0.1", p)).ok())
        .next()?;
    let port = listener.local_addr().ok()?.port();
    std::thread::Builder::new()
        .name("apb-proxy-accept".into())
        .spawn(move || accept_loop(listener, filter))
        .ok()?;
    Some(port)
}

fn accept_loop(listener: TcpListener, filter: Arc<LiveFilter>) {
    for conn in listener.incoming() {
        if let Ok(stream) = conn {
            let f = filter.clone();
            let _ = std::thread::Builder::new()
                .name("apb-proxy-conn".into())
                .spawn(move || handle_conn(stream, f));
        }
    }
}

fn handle_conn(mut client: TcpStream, filter: Arc<LiveFilter>) {
    let _ = client.set_nodelay(true);
    let _ = client.set_read_timeout(Some(IO_TIMEOUT));
    let _ = client.set_write_timeout(Some(IO_TIMEOUT));

    let head = match read_head(&mut client) {
        Some(h) => h,
        None => return,
    };
    let head_str = String::from_utf8_lossy(&head).into_owned();
    let first = head_str.lines().next().unwrap_or("").trim().to_string();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_ascii_uppercase();

    if method == "CONNECT" {
        // CONNECT host:port HTTP/1.1
        let authority = parts.next().unwrap_or("");
        let host = authority
            .rsplit_once(':')
            .map(|(h, _)| h.to_string())
            .unwrap_or_else(|| authority.to_string());
        if should_block(&filter, &host) {
            let _ = client.write_all(
                b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return;
        }
        if client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").is_err() {
            return;
        }
        let Ok(mut upstream) = TcpStream::connect(authority) else { return };
        let _ = upstream.set_nodelay(true);
        tunnel(client, upstream);
    } else {
        // Absolute-form request: GET http://host/path HTTP/1.1
        let url_part = parts.next().unwrap_or("");
        let host = extract_http_host(&head_str, url_part);
        let host_port = http_target(&host);
        if should_block(&filter, &host) {
            let body = "Blocked by APB privacy engine";
            let resp = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = client.write_all(resp.as_bytes());
            return;
        }
        let Some(mut upstream) = host_port.and_then(|t| TcpStream::connect(t).ok()) else { return };
        let _ = upstream.set_nodelay(true);
        // Forward the exact head bytes we consumed, then splice both ways so
        // any request body still in `client` flows out and the response flows
        // back without us parsing it.
        if upstream.write_all(&head).is_err() {
            return;
        }
        tunnel(client, upstream);
    }
}

/// Blind bidirectional copy until either side closes.
fn tunnel(a: TcpStream, b: TcpStream) {
    let mut a = a;
    let mut b = b;
    let Ok(mut a2) = a.try_clone() else { return };
    let Ok(mut b2) = b.try_clone() else { return };
    let up = std::thread::spawn(move || {
        let _ = std::io::copy(&mut a2, &mut b2);
        let _ = b2.shutdown(std::net::Shutdown::Write);
    });
    let _ = std::io::copy(&mut b, &mut a);
    let _ = a.shutdown(std::net::Shutdown::Both);
    let _ = up.join();
}

/// Read bytes up to and including the CRLFCRLF terminator (capped).
fn read_head(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut buf = Vec::with_capacity(8 * 1024);
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if find_head_end(&buf).is_some() {
            return Some(buf);
        }
        if buf.len() > HEAD_CAP {
            return None;
        }
    }
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn extract_http_host(head: &str, url_part: &str) -> String {
    // Prefer the authority from the absolute URL.
    if let Some(rest) = url_part.strip_prefix("http://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        if !authority.is_empty() {
            return authority.split('@').next_back().unwrap_or(authority).to_string();
        }
    }
    // Fallback: Host header.
    for line in head.lines().skip(1) {
        if let Some(v) = line.strip_prefix("Host:") {
            return v.trim().split('@').next_back().unwrap_or_else(|| v.trim()).to_string();
        }
    }
    String::new()
}

fn http_target(host: &str) -> Option<String> {
    let host = host.trim();
    if host.is_empty() {
        return None;
    }
    match host.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => {
            Some(format!("{h}:{p}"))
        }
        _ => Some(format!("{host}:80")),
    }
}

fn should_block(filter: &Arc<LiveFilter>, host: &str) -> bool {
    let host = host.trim().trim_matches(['[', ']']).to_lowercase();
    if !worth_checking(&host) {
        return false;
    }
    let snap = filter.get();
    let p = &snap.policy;
    if !(p.block_trackers || p.block_ads || p.block_malicious_domains) {
        return false;
    }
    snap.blocker.inspect(&host, p).is_some()
}

/// Never touch loopback/LAN/IP-literal targets — trackers don't live there,
/// and false positives on the local network would be infuriating.
fn worth_checking(host: &str) -> bool {
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return false;
    }
    if host.parse::<Ipv4Addr>().is_ok() || host.parse::<Ipv6Addr>().is_ok() {
        return false;
    }
    true
}

// Made by MrDuck && Ox-Alpha
