//! Unit tests for the health monitoring engine module.

use llama_herd::health::{HealthState, check_health, parse_health_response};

#[test]
fn test_unreachable_port_returns_unhealthy() {
    // Port 59999 is highly likely to be closed locally
    let state = check_health("127.0.0.1", 59999);
    assert_eq!(
        state,
        HealthState::Unhealthy,
        "Unreachable port should return Unhealthy"
    );
}

#[test]
fn test_health_state_display_formatting() {
    assert_eq!(format!("{}", HealthState::Healthy), "Healthy");
    assert_eq!(format!("{}", HealthState::Loading), "Loading");
    assert_eq!(format!("{}", HealthState::Unhealthy), "Unhealthy");
    assert_eq!(format!("{}", HealthState::Recovering), "Recovering");
}

#[test]
fn test_parse_health_response() {
    let healthy_resp =
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}";
    assert_eq!(parse_health_response(healthy_resp), HealthState::Healthy);

    let loading_resp_compact =
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"loading\"}";
    assert_eq!(
        parse_health_response(loading_resp_compact),
        HealthState::Loading
    );

    let loading_resp_spaced =
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\": \"loading\"}";
    assert_eq!(
        parse_health_response(loading_resp_spaced),
        HealthState::Loading
    );

    let service_unavailable_resp = "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\n\r\n{\"status\":\"loading\"}";
    assert_eq!(
        parse_health_response(service_unavailable_resp),
        HealthState::Loading
    );

    let internal_error_resp = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
    assert_eq!(
        parse_health_response(internal_error_resp),
        HealthState::Unhealthy
    );
}

#[test]
fn test_serde_serialization() {
    let json = serde_json::to_string(&HealthState::Healthy);
    assert!(json.is_ok(), "Serialization failed");
    if let Ok(serialized) = json {
        assert_eq!(serialized, "\"healthy\"");
        let deserialized: Result<HealthState, _> = serde_json::from_str(&serialized);
        assert!(deserialized.is_ok(), "Deserialization failed");
        if let Ok(state) = deserialized {
            assert_eq!(state, HealthState::Healthy);
        }
    }
}
