//! Direct Control Dispatcher for sending runtime control commands to `llama-server`.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Parses an HTTP response string from the control endpoint.
///
/// Returns `Ok(())` if the HTTP response status is `200 OK`, otherwise returns an `Err(String)`.
///
/// # Errors
/// Returns `Err(String)` if response is empty or status code is not 200 OK.
pub fn parse_control_response(resp: &str) -> Result<(), String> {
    let first_line = resp.lines().next().unwrap_or("");
    if first_line.is_empty() {
        return Err("Empty response from control endpoint".to_owned());
    }

    if first_line.contains("200 OK")
        || (first_line.starts_with("HTTP/") && first_line.contains(" 200 "))
    {
        Ok(())
    } else {
        Err(format!(
            "Control request failed with status line: '{first_line}'"
        ))
    }
}

/// Issues a `POST /v1/chat/completions/control` request to cancel/stop active generation on `llama-server`.
///
/// Uses [`TcpStream`] with a 1000ms timeout for connect, read, and write operations.
///
/// # Errors
/// Returns an `Err(String)` if network connection fails, times out, or the server responds with a non-200 status.
pub fn cancel_active_generation(host: &str, port: u16) -> Result<(), String> {
    let timeout = Duration::from_secs(1);

    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve address {host}:{port}: {e}"))?;

    let mut stream = None;
    for addr in addrs {
        if let Ok(s) = TcpStream::connect_timeout(&addr, timeout) {
            stream = Some(s);
            break;
        }
    }

    let Some(mut stream) = stream else {
        return Err(format!("Failed to connect to {host}:{port} within timeout"));
    };

    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("Failed to set read timeout: {e}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("Failed to set write timeout: {e}"))?;

    let payload = r#"{"action":"stop"}"#;
    let request = format!(
        "POST /v1/chat/completions/control HTTP/1.1\r\n\
         Host: {host}:{port}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         User-Agent: llama-herd\r\n\
         \r\n\
         {payload}",
        payload.len()
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed to send control request: {e}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| format!("Failed to read control response: {e}"))?;

    let resp_str = String::from_utf8_lossy(&response);
    parse_control_response(&resp_str)
}
