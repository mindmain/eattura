use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri_plugin_opener::OpenerExt;

const MAX_ENTRIES: usize = 500;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const LOG_FILE: &str = "events.jsonl";
const PREVIOUS_FILE: &str = "events.previous.jsonl";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    timestamp: i64,
    level: LogLevel,
    source: String,
    message: String,
}

#[derive(Serialize)]
pub struct LogSnapshot {
    entries: Vec<LogEntry>,
    directory: Option<String>,
}

/// Local diagnostics are independent from invoice snapshots and vault changes.
#[derive(Clone)]
pub struct AppLog {
    directory: Option<PathBuf>,
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl AppLog {
    pub fn open(directory: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&directory)?;
        let mut entries = VecDeque::new();
        for name in [PREVIOUS_FILE, LOG_FILE] {
            let path = directory.join(name);
            let file = match fs::File::open(path) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            let mut bytes = Vec::new();
            file.take(MAX_FILE_BYTES).read_to_end(&mut bytes)?;
            for line in bytes.split(|byte| *byte == b'\n') {
                // Ignore incomplete records left by an interrupted write.
                if let Ok(entry) = serde_json::from_slice::<LogEntry>(line) {
                    push_bounded(&mut entries, entry);
                }
            }
        }
        Ok(Self {
            directory: Some(directory),
            entries: Arc::new(Mutex::new(entries)),
        })
    }

    pub fn memory() -> Self {
        Self {
            directory: None,
            entries: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Callers provide diagnostic messages only, never command arguments or credentials.
    pub fn write(&self, level: LogLevel, source: &str, message: &str) -> io::Result<()> {
        let entry = LogEntry {
            timestamp: chrono::Utc::now().timestamp_millis(),
            level,
            source: source.chars().take(80).collect(),
            message: message.chars().take(2048).collect(),
        };
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| io::Error::other("Log lock unavailable"))?;
        push_bounded(&mut entries, entry.clone());
        if let Some(directory) = &self.directory {
            let mut bytes = serde_json::to_vec(&entry)?;
            bytes.push(b'\n');
            append_rotating(directory, &bytes, MAX_FILE_BYTES)?;
        }
        Ok(())
    }

    pub fn record(&self, level: LogLevel, source: &str, message: &str) {
        if let Err(error) = self.write(level, source, message) {
            // Logging must never interrupt an invoice operation or recurse into itself.
            eprintln!("[app-log] unable to persist diagnostics: {error}");
        }
    }

    fn snapshot(&self) -> io::Result<LogSnapshot> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| io::Error::other("Log lock unavailable"))?;
        Ok(LogSnapshot {
            entries: entries.iter().rev().cloned().collect(),
            directory: self
                .directory
                .as_ref()
                .map(|path| path.display().to_string()),
        })
    }
}

fn push_bounded(entries: &mut VecDeque<LogEntry>, entry: LogEntry) {
    entries.push_back(entry);
    while entries.len() > MAX_ENTRIES {
        entries.pop_front();
    }
}

fn append_rotating(directory: &Path, bytes: &[u8], max_bytes: u64) -> io::Result<()> {
    let path = directory.join(LOG_FILE);
    let size = match fs::metadata(&path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error),
    };
    if size > 0 && size + bytes.len() as u64 + 1 > max_bytes {
        let previous = directory.join(PREVIOUS_FILE);
        match fs::remove_file(&previous) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(&path, previous)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    if file.metadata()?.len() > 0 {
        file.seek(SeekFrom::End(-1))?;
        let mut last_byte = [0];
        file.read_exact(&mut last_byte)?;
        // Separate the next record if the previous process left a partial line.
        if last_byte[0] != b'\n' {
            file.write_all(b"\n")?;
        }
    }
    file.write_all(bytes)
}

#[tauri::command]
pub fn get_app_logs(log: tauri::State<'_, AppLog>) -> Result<LogSnapshot, String> {
    log.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn write_app_log(
    log: tauri::State<'_, AppLog>,
    level: LogLevel,
    source: String,
    message: String,
) -> Result<(), String> {
    log.write(level, &source, &message)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_app_log_directory(
    app: tauri::AppHandle,
    log: tauri::State<'_, AppLog>,
) -> Result<(), String> {
    let directory = log
        .directory
        .as_ref()
        .ok_or("Persistent logging unavailable")?;
    app.opener()
        .open_path(directory.display().to_string(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_bounded_history_and_skips_incomplete_records() {
        let directory = tempfile::tempdir().unwrap();
        let log = AppLog::open(directory.path().to_path_buf()).unwrap();
        for index in 0..MAX_ENTRIES + 10 {
            log.write(LogLevel::Info, "test", &index.to_string())
                .unwrap();
        }
        OpenOptions::new()
            .append(true)
            .open(directory.path().join(LOG_FILE))
            .unwrap()
            .write_all(b"{incomplete")
            .unwrap();

        let reopened = AppLog::open(directory.path().to_path_buf()).unwrap();
        let restored = reopened.snapshot().unwrap();
        assert_eq!(restored.entries.len(), MAX_ENTRIES);
        assert_eq!(restored.entries[0].message, "509");
        assert_eq!(restored.entries[MAX_ENTRIES - 1].message, "10");
        reopened.write(LogLevel::Info, "test", "Recovered").unwrap();
        let recovered = AppLog::open(directory.path().to_path_buf())
            .unwrap()
            .snapshot()
            .unwrap();
        assert_eq!(recovered.entries[0].message, "Recovered");
    }

    #[test]
    fn rotates_files_without_growing_the_archive() {
        let directory = tempfile::tempdir().unwrap();
        append_rotating(directory.path(), b"first\n", 12).unwrap();
        append_rotating(directory.path(), b"second\n", 12).unwrap();
        append_rotating(directory.path(), b"third\n", 12).unwrap();
        assert_eq!(
            fs::read(directory.path().join(PREVIOUS_FILE)).unwrap(),
            b"second\n"
        );
        assert_eq!(
            fs::read(directory.path().join(LOG_FILE)).unwrap(),
            b"third\n"
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn isolates_local_logs_and_keeps_diagnostics_when_disk_writes_fail() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let log = AppLog::open(first.path().to_path_buf()).unwrap();
        let other = AppLog::open(second.path().to_path_buf()).unwrap();
        fs::create_dir(first.path().join(LOG_FILE)).unwrap();
        assert!(
            log.write(LogLevel::Error, "storage", "Disk unavailable")
                .is_err()
        );
        assert_eq!(
            log.snapshot().unwrap().entries[0].message,
            "Disk unavailable"
        );
        assert!(other.snapshot().unwrap().entries.is_empty());
    }
}
