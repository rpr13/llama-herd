//! Unit tests for the direct control dispatcher module.

use llama_herd::control::{cancel_active_generation, parse_control_response};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[test]
fn test_unreachable_port_returns_err() {
    // Port 59998 is expected to be closed locally
    let result = cancel_active_generation("127.0.0.1", 59998);
    assert!(result.is_err(), "Unreachable port should return Err");
}

#[test]
fn test_parse_control_response_status_codes() {
    let ok_resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}";
    assert!(parse_control_response(ok_resp).is_ok());

    let ok_resp_simple = "HTTP/1.0 200 OK\r\n\r\n";
    assert!(parse_control_response(ok_resp_simple).is_ok());

    let err_resp_500 = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
    assert!(parse_control_response(err_resp_500).is_err());

    let err_resp_404 = "HTTP/1.1 404 Not Found\r\n\r\n";
    assert!(parse_control_response(err_resp_404).is_err());

    let empty_resp = "";
    assert!(parse_control_response(empty_resp).is_err());
}

#[test]
fn test_cancel_active_generation_payload_and_server_response() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test TCP listener");
    let addr = listener.local_addr().expect("Failed to get local addr");

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("Failed to set read timeout");

        let mut buffer = [0u8; 1024];
        let bytes_read = stream.read(&mut buffer).expect("Failed to read request");
        let req_str = String::from_utf8_lossy(&buffer[..bytes_read]);

        // Verify request method and endpoint
        assert!(
            req_str.starts_with("POST /v1/chat/completions/control HTTP/1.1"),
            "Expected POST request to control endpoint, got: {req_str}"
        );

        // Verify headers
        assert!(
            req_str.contains("Content-Type: application/json"),
            "Expected Content-Type header in request"
        );

        // Verify payload body
        assert!(
            req_str.ends_with("{\"action\":\"stop\"}"),
            "Expected body to be {{\"action\":\"stop\"}}, request was: {req_str}"
        );

        // Send 200 OK response back
        let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}";
        stream
            .write_all(response.as_bytes())
            .expect("Failed to send response");
    });

    let client_result = cancel_active_generation("127.0.0.1", addr.port());
    assert!(
        client_result.is_ok(),
        "Expected cancel_active_generation to return Ok, got: {client_result:?}"
    );

    server_handle.join().expect("Server thread panicked");
}
