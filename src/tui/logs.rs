use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use ratatui::style::{Color, Modifier, Style};

/// A single styled fragment within a log line.
#[derive(Clone, Debug)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

/// A parsed log line containing one or more styled spans.
#[derive(Clone, Debug)]
pub struct LogLine {
    pub spans: Vec<StyledSpan>,
}

pub struct ActiveServer {
    pub child: Child,
    pub logs: Arc<Mutex<Vec<LogLine>>>,
    pub raw_history: Arc<Mutex<Vec<String>>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl ActiveServer {
    pub fn spawn(params: &[String], cwd: &Path) -> Result<Self, std::io::Error> {
        let mut cmd = Command::new(&params[0]);
        cmd.args(&params[1..])
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("Failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("Failed to capture stderr"))?;

        let logs = Arc::new(Mutex::new(Vec::new()));
        let raw_history = Arc::new(Mutex::new(Vec::new()));
        let is_running = Arc::new(Mutex::new(true));

        // Spawn stdout thread
        {
            let logs = logs.clone();
            let raw_history = raw_history.clone();
            let is_running = is_running.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line_res in reader.lines() {
                    if !*is_running.lock().unwrap() {
                        break;
                    }
                    if let Ok(line) = line_res {
                        let parsed = parse_ansi_line(&line);
                        raw_history.lock().unwrap().push(line);
                        logs.lock().unwrap().push(parsed);
                    }
                }
            });
        }

        // Spawn stderr thread
        {
            let logs = logs.clone();
            let raw_history = raw_history.clone();
            let is_running = is_running.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line_res in reader.lines() {
                    if !*is_running.lock().unwrap() {
                        break;
                    }
                    if let Ok(line) = line_res {
                        let parsed = parse_ansi_line(&line);
                        raw_history.lock().unwrap().push(line);
                        logs.lock().unwrap().push(parsed);
                    }
                }
            });
        }

        Ok(ActiveServer {
            child,
            logs,
            raw_history,
            is_running,
        })
    }

    pub fn kill(&mut self) {
        *self.is_running.lock().unwrap() = false;
        #[cfg(target_os = "windows")]
        {
            let pid = self.child.id();
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string(), "/T"])
                .output();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// Parse a line containing ANSI escape sequences into styled spans.
///
/// Supports SGR codes: reset (0), bold (1), colors 30-37 (fg), 40-47 (bg),
/// and 256-color via 38;5;N / 48;5;N.
pub fn parse_ansi_line(input: &str) -> LogLine {
    let mut spans = Vec::new();
    let mut current_style = Style::default();
    let mut buf = String::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Detect ESC [ sequence
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            if !buf.is_empty() {
                spans.push(StyledSpan {
                    text: std::mem::take(&mut buf),
                    style: current_style,
                });
            }

            i += 2; // skip ESC [
            let mut params = String::new();
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
                params.push(bytes[i] as char);
                i += 1;
            }
            if i < len {
                let terminator = bytes[i] as char;
                i += 1; // skip terminator

                if terminator == 'm' {
                    current_style = apply_sgr(&params, current_style);
                }
                // Other terminators (K, H, J, etc.) are cursor/erase ops — skip them
            }
        } else {
            buf.push(bytes[i] as char);
            i += 1;
        }
    }

    if !buf.is_empty() {
        spans.push(StyledSpan {
            text: buf,
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
            37 => style = style.fg(Color::White),
            39 => style = style.fg(Color::Reset),

            // Bright foreground colors
            90 => style = style.fg(Color::DarkGray),
            91 => style = style.fg(Color::LightRed),
            92 => style = style.fg(Color::LightGreen),
            93 => style = style.fg(Color::LightYellow),
            94 => style = style.fg(Color::LightBlue),
            95 => style = style.fg(Color::LightMagenta),
            96 => style = style.fg(Color::LightCyan),
            97 => style = style.fg(Color::White),

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
