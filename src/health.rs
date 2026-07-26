//! Health monitoring engine for inspecting local `llama-server` instances.

use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Health state of a `llama-server` instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    /// Server is healthy and ready for inference requests.
    Healthy,
    /// Server is loading model weights or starting up.
    Loading,
    /// Server is unreachable or in an error state.
    Unhealthy,
    /// Server is recovering from an issue.
    Recovering,
}

impl fmt::Display for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "Healthy"),
            Self::Loading => write!(f, "Loading"),
            Self::Unhealthy => write!(f, "Unhealthy"),
            Self::Recovering => write!(f, "Recovering"),
        }
    }
}

/// Parses an HTTP response string into a corresponding [`HealthState`].
#[must_use]
pub fn parse_health_response(resp: &str) -> HealthState {
    let first_line = resp.lines().next().unwrap_or("");
    let is_200 = first_line.contains("200");
    let is_503 = first_line.contains("503") || resp.contains("503 Service Unavailable");

    let is_loading_payload = resp.contains(r#""status":"loading""#)
        || resp.contains(r#""status": "loading""#)
        || resp.contains(r#""status":"LOADING""#)
        || resp.contains(r#""status": "LOADING""#);

    if is_200 {
        if is_loading_payload {
            HealthState::Loading
        } else {
            HealthState::Healthy
        }
    } else if is_503 || is_loading_payload {
        HealthState::Loading
    } else {
        HealthState::Unhealthy
    }
}

/// Checks the health status of a `llama-server` endpoint at `http://<host>:<port>/health`.
///
/// Connects via [`TcpStream`] with a 500ms timeout for socket connect, read, and write operations.
#[must_use]
pub fn check_health(host: &str, port: u16) -> HealthState {
    let timeout = Duration::from_millis(500);

    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return HealthState::Unhealthy;
    };

    let mut stream = None;
    for addr in addrs {
        if let Ok(s) = TcpStream::connect_timeout(&addr, timeout) {
            stream = Some(s);
            break;
        }
    }

    let Some(mut stream) = stream else {
        return HealthState::Unhealthy;
    };

    if stream.set_read_timeout(Some(timeout)).is_err()
        || stream.set_write_timeout(Some(timeout)).is_err()
    {
        return HealthState::Unhealthy;
    }

    let request = format!(
        "GET /health HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\nUser-Agent: llama-herd\r\n\r\n"
    );

    if stream.write_all(request.as_bytes()).is_err() {
        return HealthState::Unhealthy;
    }

    let mut response = Vec::new();
    if stream.read_to_end(&mut response).is_err() {
        return HealthState::Unhealthy;
    }

    let resp_str = String::from_utf8_lossy(&response);
    parse_health_response(&resp_str)
}
