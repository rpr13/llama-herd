use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use ratatui::style::{Color, Modifier, Style};

/// The maximum number of log lines preserved in the memory buffer.
pub const MAX_LOGS: usize = 5000;

/// Process supervisor configuration holding original launch parameters and target endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupervisorConfig {
    /// Original command line parameters used to spawn llama-server.
    pub params: Vec<String>,
    /// Working directory for the process.
    pub cwd: PathBuf,
    /// Optional model name associated with this process.
    pub model_name: Option<String>,
    /// Server endpoint host address.
    pub host: String,
    /// Server endpoint port.
    pub port: u16,
}

impl SupervisorConfig {
    /// Constructs a `SupervisorConfig` from parameters, working directory, and optional model name,
    /// parsing `--host` and `--port` if present in `params`.
    #[must_use]
    pub fn new(params: Vec<String>, cwd: PathBuf, model_name: Option<String>) -> Self {
        let mut host = "127.0.0.1".to_owned();
        let mut port = 8080;

        let mut idx = 0;
        while idx < params.len() {
            if params[idx] == "--host" && idx + 1 < params.len() {
                host.clone_from(&params[idx + 1]);
            } else if params[idx] == "--port" && idx + 1 < params.len() {
                if let Ok(p) = params[idx + 1].parse::<u16>() {
                    port = p;
                }
            }
            idx += 1;
        }

        Self {
            params,
            cwd,
            model_name,
            host,
            port,
        }
    }
}

/// A single styled fragment within a log line.
#[derive(Clone, Debug)]
pub struct StyledSpan {
    /// String slice text.
    pub text: String,
    /// Terminal character style details.
    pub style: Style,
}

/// A parsed log line containing one or more styled spans.
#[derive(Clone, Debug)]
pub struct LogLine {
    /// The constituent styled spans.
    pub spans: Vec<StyledSpan>,
}

/// Runtime indicators and resource utilization metrics of the active llama-server.
#[derive(Clone, Debug, Default)]
pub struct ServerMetrics {
    /// Server status string (e.g., "LOADING", "RUNNING", "HEALTHY", "UNHEALTHY", "RECOVERING", "STOPPED", "ERROR").
    pub status: String,
    /// Subprocess PID if available.
    pub pid: Option<u32>,
    /// Flag indicating if server is running in Router Mode.
    pub is_router: bool,
    /// Maximum models capacity configured in Router Mode.
    pub max_models: Option<usize>,
    /// Name of the active model currently routed.
    pub active_model: Option<String>,
    /// Port number the server is listening on.
    pub active_port: Option<u16>,
    /// Used and total VRAM in MiB.
    pub vram_usage: Option<(u64, u64)>,
    /// Used and total system RAM in MiB.
    pub ram_usage: Option<(u64, u64)>,
}

/// Subprocess manager wrapper orchestrating log streaming and metric updates.
pub struct ActiveServer {
    /// Shared reference to the child subprocess.
    pub child: Arc<Mutex<Child>>,
    /// Shared buffer of parsed log lines.
    pub logs: Arc<Mutex<VecDeque<LogLine>>>,
    /// Shared buffer of raw stdout/stderr outputs.
    pub raw_history: Arc<Mutex<VecDeque<String>>>,
    /// Shared running state indicator.
    pub is_running: Arc<Mutex<bool>>,
    /// Shared metrics instance.
    pub metrics: Arc<Mutex<ServerMetrics>>,
    /// Process supervisor configuration holding original launch parameters and target endpoint.
    pub config: SupervisorConfig,
    /// Flag indicating whether server stop was manually requested.
    pub manual_stop: Arc<Mutex<bool>>,
}

/// Type alias for [`ActiveServer`].
pub type LogStream = ActiveServer;

impl std::fmt::Debug for ActiveServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveServer")
            .field("logs", &self.logs)
            .field("raw_history", &self.raw_history)
            .field("is_running", &self.is_running)
            .field("metrics", &self.metrics)
            .field("config", &self.config)
            .field("manual_stop", &self.manual_stop)
            .finish_non_exhaustive()
    }
}

impl ActiveServer {
    /// Spawns a new llama-server subprocess and launches streaming threads.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the subprocess fails to spawn or if IO pipes cannot be captured.
    ///
    /// # Panics
    ///
    /// Panics if any of the internal mutexes are poisoned during log streaming or status monitoring.
    pub fn spawn(
        params: &[String],
        cwd: &Path,
        model_name: Option<String>,
        event_tx: Option<std::sync::mpsc::Sender<crate::tui::TuiEvent>>,
    ) -> Result<Self, std::io::Error> {
        let config = SupervisorConfig::new(params.to_vec(), cwd.to_path_buf(), model_name);
        Self::spawn_supervised(config, event_tx)
    }

    /// Spawns a new process managed by the process supervisor using the provided [`SupervisorConfig`].
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the subprocess fails to spawn or if IO pipes cannot be captured.
    ///
    /// # Panics
    ///
    /// Panics if any of the internal mutexes are poisoned during log streaming or status monitoring.
    pub fn spawn_supervised(
        config: SupervisorConfig,
        event_tx: Option<std::sync::mpsc::Sender<crate::tui::TuiEvent>>,
    ) -> Result<Self, std::io::Error> {
        let mut cmd = Command::new(&config.params[0]);
        cmd.args(&config.params[1..])
            .current_dir(&config.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        crate::launcher::add_active_pid(child.id());

        let stdout = if let Some(out) = child.stdout.take() {
            out
        } else {
            let _ = child.kill();
            let _ = child.wait();
            crate::launcher::remove_active_pid(child.id());
            return Err(std::io::Error::other("Failed to capture stdout"));
        };
        let stderr = if let Some(err) = child.stderr.take() {
            err
        } else {
            let _ = child.kill();
            let _ = child.wait();
            crate::launcher::remove_active_pid(child.id());
            return Err(std::io::Error::other("Failed to capture stderr"));
        };

        let logs = Arc::new(Mutex::new(VecDeque::new()));
        let raw_history = Arc::new(Mutex::new(VecDeque::new()));
        let is_running = Arc::new(Mutex::new(true));
        let manual_stop = Arc::new(Mutex::new(false));
        let is_router = config.params.iter().any(|arg| arg == "--models-preset");

        let mut max_models = None;
        let mut idx = 0;
        while idx < config.params.len() {
            if config.params[idx] == "--models-max" && idx + 1 < config.params.len() {
                max_models = config.params[idx + 1].parse().ok();
            }
            idx += 1;
        }

        let metrics = Arc::new(Mutex::new(ServerMetrics {
            status: "LOADING".to_owned(),
            pid: Some(child.id()),
            is_router,
            max_models,
            active_model: if is_router {
                None
            } else {
                config.model_name.clone()
            },
            active_port: Some(config.port),
            vram_usage: None,
            ram_usage: None,
        }));

        let child = Arc::new(Mutex::new(child));

        // Spawn status monitoring thread
        {
            let is_running = Arc::clone(&is_running);
            let metrics = Arc::clone(&metrics);
            let child_ref = Arc::clone(&child);
            let config_clone = config.clone();
            let manual_stop_clone = Arc::clone(&manual_stop);
            let logs_clone = Arc::clone(&logs);
            let raw_history_clone = Arc::clone(&raw_history);
            let event_tx_clone = event_tx.clone();
            thread::spawn(move || {
                Self::monitor_status(
                    &is_running,
                    &metrics,
                    &child_ref,
                    &config_clone,
                    &manual_stop_clone,
                    &logs_clone,
                    &raw_history_clone,
                    event_tx_clone,
                );
            });
        }

        // Spawn stdout thread
        {
            let logs = Arc::clone(&logs);
            let raw_history = Arc::clone(&raw_history);
            let event_tx = event_tx.clone();
            let metrics = Arc::clone(&metrics);
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                Self::process_log_stream(reader, &metrics, &logs, &raw_history, event_tx.as_ref());
            });
        }

        // Spawn stderr thread
        {
            let logs = Arc::clone(&logs);
            let raw_history = Arc::clone(&raw_history);
            let event_tx = event_tx;
            let metrics = Arc::clone(&metrics);
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                Self::process_log_stream(reader, &metrics, &logs, &raw_history, event_tx.as_ref());
            });
        }

        Ok(Self {
            child,
            logs,
            raw_history,
            is_running,
            metrics,
            config,
            manual_stop,
        })
    }

    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::needless_pass_by_value
    )]
    fn monitor_status(
        is_running: &Arc<Mutex<bool>>,
        metrics: &Arc<Mutex<ServerMetrics>>,
        child_ref: &Arc<Mutex<Child>>,
        config: &SupervisorConfig,
        manual_stop: &Arc<Mutex<bool>>,
        logs: &Arc<Mutex<VecDeque<LogLine>>>,
        raw_history: &Arc<Mutex<VecDeque<String>>>,
        event_tx: Option<std::sync::mpsc::Sender<crate::tui::TuiEvent>>,
    ) {
        let mut sys = sysinfo::System::new();
        let mut vram_counter = 0u64;

        while *is_running.lock().expect("is_running lock poisoned") {
            let exit_status = {
                let mut child_lock = child_ref.lock().expect("child lock poisoned");
                child_lock.try_wait()
            };

            let exit_occurred = match exit_status {
                Ok(Some(_status)) => true,
                Err(_) => true,
                Ok(None) => false,
            };

            if exit_occurred {
                let is_manual = *manual_stop.lock().expect("manual_stop lock poisoned");
                if is_manual {
                    let pid = child_ref.lock().expect("child lock poisoned").id();
                    crate::launcher::remove_active_pid(pid);
                    if let Ok(mut m_lock) = metrics.lock() {
                        "STOPPED".clone_into(&mut m_lock.status);
                    }
                    *is_running.lock().expect("is_running lock poisoned") = false;
                    break;
                }

                // Unexpected process exit & manual_stop == false => Auto-recovery!
                {
                    let mut m_lock = metrics.lock().expect("metrics lock poisoned");
                    "RECOVERING".clone_into(&mut m_lock.status);
                }

                let supervisor_msg =
                    "[SUPERVISOR] Process crash detected. Auto-restarting with original parameters..."
                        .to_owned();
                let parsed = parse_ansi_line(&supervisor_msg);
                {
                    let mut hist_lock = raw_history.lock().expect("raw_history lock poisoned");
                    hist_lock.push_back(supervisor_msg);
                    if hist_lock.len() > MAX_LOGS {
                        hist_lock.pop_front();
                    }
                }
                {
                    let mut logs_lock = logs.lock().expect("logs lock poisoned");
                    logs_lock.push_back(parsed);
                    if logs_lock.len() > MAX_LOGS {
                        logs_lock.pop_front();
                    }
                }
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(crate::tui::TuiEvent::LogReceived);
                }

                let old_pid = child_ref.lock().expect("child lock poisoned").id();
                crate::launcher::remove_active_pid(old_pid);

                // Re-spawn process using EXACT original parameters (`params` and `cwd`) stored in `SupervisorConfig`
                let mut cmd = Command::new(&config.params[0]);
                cmd.args(&config.params[1..])
                    .current_dir(&config.cwd)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                match cmd.spawn() {
                    Ok(mut new_child) => {
                        let new_pid = new_child.id();
                        crate::launcher::add_active_pid(new_pid);

                        if let Ok(mut m_lock) = metrics.lock() {
                            m_lock.pid = Some(new_pid);
                        }

                        let stdout = new_child.stdout.take();
                        let stderr = new_child.stderr.take();

                        {
                            let mut child_lock = child_ref.lock().expect("child lock poisoned");
                            *child_lock = new_child;
                        }

                        // Re-attach stdout stream reader
                        if let Some(out) = stdout {
                            let logs_c = Arc::clone(logs);
                            let raw_history_c = Arc::clone(raw_history);
                            let event_tx_c = event_tx.clone();
                            let metrics_c = Arc::clone(metrics);
                            thread::spawn(move || {
                                let reader = BufReader::new(out);
                                Self::process_log_stream(
                                    reader,
                                    &metrics_c,
                                    &logs_c,
                                    &raw_history_c,
                                    event_tx_c.as_ref(),
                                );
                            });
                        }

                        // Re-attach stderr stream reader
                        if let Some(err) = stderr {
                            let logs_c = Arc::clone(logs);
                            let raw_history_c = Arc::clone(raw_history);
                            let event_tx_c = event_tx.clone();
                            let metrics_c = Arc::clone(metrics);
                            thread::spawn(move || {
                                let reader = BufReader::new(err);
                                Self::process_log_stream(
                                    reader,
                                    &metrics_c,
                                    &logs_c,
                                    &raw_history_c,
                                    event_tx_c.as_ref(),
                                );
                            });
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("[SUPERVISOR] Auto-restart failed: {e}");
                        let parsed = parse_ansi_line(&err_msg);
                        {
                            let mut hist_lock =
                                raw_history.lock().expect("raw_history lock poisoned");
                            hist_lock.push_back(err_msg);
                            if hist_lock.len() > MAX_LOGS {
                                hist_lock.pop_front();
                            }
                        }
                        {
                            let mut logs_lock = logs.lock().expect("logs lock poisoned");
                            logs_lock.push_back(parsed);
                            if logs_lock.len() > MAX_LOGS {
                                logs_lock.pop_front();
                            }
                        }
                        if let Ok(mut m_lock) = metrics.lock() {
                            "ERROR".clone_into(&mut m_lock.status);
                        }
                        *is_running.lock().expect("is_running lock poisoned") = false;
                        break;
                    }
                }
                // Sleep briefly before resuming monitoring loop
                thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }

            // Child is alive and running: query health status via REST API polling
            let health = crate::health::check_health(&config.host, config.port);
            if let Ok(mut m_lock) = metrics.lock() {
                match health {
                    crate::health::HealthState::Healthy => {
                        "HEALTHY".clone_into(&mut m_lock.status);
                    }
                    crate::health::HealthState::Loading => {
                        "LOADING".clone_into(&mut m_lock.status);
                    }
                    crate::health::HealthState::Unhealthy => {
                        if m_lock.status != "LOADING" && m_lock.status != "RECOVERING" {
                            "UNHEALTHY".clone_into(&mut m_lock.status);
                        }
                    }
                    crate::health::HealthState::Recovering => {
                        "RECOVERING".clone_into(&mut m_lock.status);
                    }
                }
            }

            // Query RAM and VRAM usage every 2 seconds
            if vram_counter % 2 == 0 {
                sys.refresh_memory();
                let total_ram = sys.total_memory() / 1024 / 1024;
                let used_ram = sys.used_memory() / 1024 / 1024;
                let vram = query_vram();
                if let Ok(mut m_lock) = metrics.lock() {
                    m_lock.ram_usage = Some((used_ram, total_ram));
                    m_lock.vram_usage = vram;
                }
            }
            vram_counter = vram_counter.wrapping_add(1);

            for _ in 0..10 {
                if !*is_running.lock().expect("is_running lock poisoned") {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    fn process_log_stream<R: BufRead>(
        reader: R,
        metrics: &Mutex<ServerMetrics>,
        logs: &Mutex<VecDeque<LogLine>>,
        raw_history: &Mutex<VecDeque<String>>,
        event_tx: Option<&std::sync::mpsc::Sender<crate::tui::TuiEvent>>,
    ) {
        for line in reader.lines().map_while(Result::ok) {
            if line.contains("update_slots: all slots are idle") {
                continue;
            }

            let startup = parse_startup_status(&line);
            let instance = parse_spawning_instance(&line);
            let proxy = parse_proxy_request(&line);
            let active = parse_active_model(&line);

            if startup || instance.is_some() || proxy.is_some() || active.is_some() {
                let mut m_lock = metrics.lock().expect("metrics lock poisoned");
                if startup && m_lock.status == "LOADING" {
                    "HEALTHY".clone_into(&mut m_lock.status);
                }
                if let Some((model, port)) = instance {
                    m_lock.active_model = Some(model);
                    m_lock.active_port = Some(port);
                    "HEALTHY".clone_into(&mut m_lock.status);
                } else if let Some((model, port)) = proxy {
                    m_lock.active_model = Some(model);
                    m_lock.active_port = Some(port);
                    "HEALTHY".clone_into(&mut m_lock.status);
                } else if let Some(model) = active {
                    m_lock.active_model = Some(model);
                    if m_lock.status == "LOADING" {
                        "HEALTHY".clone_into(&mut m_lock.status);
                    }
                }
            }

            if line.contains("proxy_reques: proxying request to model") {
                continue;
            }
            let parsed = parse_ansi_line(&line);
            {
                let mut hist_lock = raw_history.lock().expect("raw_history lock poisoned");
                hist_lock.push_back(line.clone());
                if hist_lock.len() > MAX_LOGS {
                    hist_lock.pop_front();
                }
            }
            {
                let mut logs_lock = logs.lock().expect("logs lock poisoned");
                logs_lock.push_back(parsed);
                if logs_lock.len() > MAX_LOGS {
                    logs_lock.pop_front();
                }
            }
            if let Some(tx) = event_tx {
                let _ = tx.send(crate::tui::TuiEvent::LogReceived);
            }
        }
    }

    /// Kills the active server subprocess and cleans up its tracking PID.
    ///
    /// # Panics
    ///
    /// Panics if the internal `is_running` lock is poisoned.
    pub fn kill(&mut self) {
        if let Ok(mut stop) = self.manual_stop.lock() {
            *stop = true;
        }
        if let Ok(mut m_lock) = self.metrics.lock() {
            "STOPPED".clone_into(&mut m_lock.status);
        }
        let mut was_running = self
            .is_running
            .lock()
            .expect("Failed to lock is_running state");
        if !*was_running {
            return;
        }
        *was_running = false;
        drop(was_running);

        let pid = self.child.lock().ok().map(|mut child| {
            let pid = child.id();
            if matches!(child.try_wait(), Ok(None)) {
                #[cfg(target_os = "windows")]
                {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string(), "/T"])
                        .output();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = child.kill();
                }
                let _ = child.wait();
            }
            pid
        });
        if let Some(pid) = pid {
            crate::launcher::remove_active_pid(pid);
        }
    }
}

/// Parse a line containing ANSI escape sequences into styled spans.
///
/// Supports SGR codes: reset (0), bold (1), colors 30-37 (fg), 40-47 (bg),
/// and 256-color via 38;5;N / 48;5;N.
#[must_use]
pub fn parse_ansi_line(input: &str) -> LogLine {
    let mut spans = Vec::new();
    let mut current_style = Style::default();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut last_start = 0;

    while i < len {
        // Detect ESC [ sequence
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            if i > last_start {
                spans.push(StyledSpan {
                    text: input[last_start..i].to_string(),
                    style: current_style,
                });
            }

            i += 2; // skip ESC [
            let params_start = i;
            while i < len
                && bytes[i] != b'm'
                && bytes[i] != b'K'
                && bytes[i] != b'H'
                && bytes[i] != b'J'
                && bytes[i] != b'A'
                && bytes[i] != b'B'
                && bytes[i] != b'C'
                && bytes[i] != b'D'
            {
                i += 1;
            }
            let params = &input[params_start..i];
            if i < len {
                let terminator = bytes[i];
                i += 1; // skip terminator

                if terminator == b'm' {
                    current_style = apply_sgr(params, current_style);
                }
                // Other terminators (K, H, J, etc.) are cursor/erase ops — skip them
            }
            last_start = i;
        } else {
            i += 1;
        }
    }

    if len > last_start {
        spans.push(StyledSpan {
            text: input[last_start..len].to_string(),
            style: current_style,
        });
    }

    // If the line was completely empty (no text at all), push an empty span
    if spans.is_empty() {
        spans.push(StyledSpan {
            text: String::new(),
            style: Style::default(),
        });
    }

    LogLine { spans }
}

/// Apply SGR (Select Graphic Rendition) parameter codes to an existing style.
#[must_use]
pub fn apply_sgr(params: &str, base: Style) -> Style {
    let mut style = base;
    let codes: Vec<&str> = params.split(';').collect();
    let mut idx = 0;

    while idx < codes.len() {
        let code = codes[idx].parse::<u16>().unwrap_or(0);
        match code {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            7 => style = style.add_modifier(Modifier::REVERSED),
            9 => style = style.add_modifier(Modifier::CROSSED_OUT),

            // Standard foreground colors
            30 => style = style.fg(Color::Black),
            31 => style = style.fg(Color::Red),
            32 => style = style.fg(Color::Green),
            33 => style = style.fg(Color::Yellow),
            34 => style = style.fg(Color::Blue),
            35 => style = style.fg(Color::Magenta),
            36 => style = style.fg(Color::Cyan),
            37 | 97 => style = style.fg(Color::White),
            39 => style = style.fg(Color::Reset),

            // Bright foreground colors
            90 => style = style.fg(Color::DarkGray),
            91 => style = style.fg(Color::LightRed),
            92 => style = style.fg(Color::LightGreen),
            93 => style = style.fg(Color::LightYellow),
            94 => style = style.fg(Color::LightBlue),
            95 => style = style.fg(Color::LightMagenta),
            96 => style = style.fg(Color::LightCyan),

            // Standard background colors
            40 => style = style.bg(Color::Black),
            41 => style = style.bg(Color::Red),
            42 => style = style.bg(Color::Green),
            43 => style = style.bg(Color::Yellow),
            44 => style = style.bg(Color::Blue),
            45 => style = style.bg(Color::Magenta),
            46 => style = style.bg(Color::Cyan),
            47 => style = style.bg(Color::White),
            49 => style = style.bg(Color::Reset),

            // 256-color: 38;5;N (fg) or 48;5;N (bg)
            38 if idx + 2 < codes.len() && codes[idx + 1] == "5" => {
                if let Ok(n) = codes[idx + 2].parse::<u8>() {
                    style = style.fg(Color::Indexed(n));
                }
                idx += 2;
            }
            48 if idx + 2 < codes.len() && codes[idx + 1] == "5" => {
                if let Ok(n) = codes[idx + 2].parse::<u8>() {
                    style = style.bg(Color::Indexed(n));
                }
                idx += 2;
            }

            _ => {} // ignore unknown codes
        }
        idx += 1;
    }

    style
}
/// Parses log lines to check if the server startup has completed successfully.
#[must_use]
pub fn parse_startup_status(line: &str) -> bool {
    line.contains("HTTP server running")
        || line.contains("binding port")
        || line.contains("Available models")
        || line.contains("running without SSL")
}

/// Parses log lines to extract routed model name and port number in Router Mode.
#[must_use]
pub fn parse_spawning_instance(line: &str) -> Option<(String, u16)> {
    if let Some(pos) = line.find("spawning server instance with name=") {
        let rest = &line[pos + 35..];
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if let Some(&model) = parts.first() {
            let mut port = None;
            for i in 0..parts.len() {
                if parts[i] == "port" && i + 1 < parts.len() {
                    port = parts[i + 1].parse::<u16>().ok();
                }
            }
            return Some((model.to_owned(), port.unwrap_or(0)));
        }
    }
    None
}

/// Parses proxy logs to check which model and port a request is routed to.
#[must_use]
pub fn parse_proxy_request(line: &str) -> Option<(String, u16)> {
    if let Some(pos) = line.find("proxy_reques: proxying request to model ") {
        let rest = &line[pos + 40..];
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if let Some(&model) = parts.first() {
            let mut port = None;
            for i in 0..parts.len() {
                if parts[i] == "port" && i + 1 < parts.len() {
                    port = parts[i + 1].parse::<u16>().ok();
                }
            }
            return Some((model.to_owned(), port.unwrap_or(0)));
        }
    }
    None
}

/// Parses log lines to extract the active model name.
#[must_use]
pub fn parse_active_model(line: &str) -> Option<String> {
    if let Some(pos) = line.find("ensure_model: waiting until model name=") {
        let rest = &line[pos + 39..];
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if let Some(model) = parts.first() {
            return Some(model.to_string());
        }
    }
    if let Some(pos) = line.find("proxy_reques: proxying request to model ") {
        let rest = &line[pos + 40..];
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if let Some(model) = parts.first() {
            return Some(model.to_string());
        }
    }
    if let Some(pos) = line.find("spawning server instance with name=") {
        let rest = &line[pos + 35..];
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if let Some(model) = parts.first() {
            return Some(model.to_string());
        }
    }
    None
}

/// Queries VRAM utilization stats using the `nvidia-smi` command.
#[must_use]
pub fn query_vram() -> Option<(u64, u64)> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut total_used = 0;
    let mut total_limit = 0;
    let mut found = false;

    for line in stdout_str.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 2 {
            let parsed = (
                parts[0].trim().parse::<u64>(),
                parts[1].trim().parse::<u64>(),
            );
            if let (Ok(used), Ok(total)) = parsed {
                total_used += used;
                total_limit += total;
                found = true;
            }
        }
    }

    if found {
        Some((total_used, total_limit))
    } else {
        None
    }
}
