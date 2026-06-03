use std::fs;
use std::path::Path;
use serde::{de::DeserializeOwned, Serialize};

/// Reads JSON `<dir>/<file>`. Returns `T::default()` if the file is missing or unparsable.
pub fn read_json<T: DeserializeOwned + Default>(dir: &Path, file: &str) -> T {
    let path = dir.join(file);
    let Ok(text) = fs::read_to_string(&path) else { return T::default() };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("[storage] failed to parse {} — using defaults: {e}", path.display());
        T::default()
    })
}

/// Writes `value` as pretty JSON to `<dir>/<file>`, creating `dir` if needed.
pub fn write_json<T: Serialize>(dir: &Path, file: &str, value: &T) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(dir.join(file), text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::models::{FoTask, PomodoroConfig};

    fn temp_dir(tag: &str) -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("fopomoro_test_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn missing_file_returns_default_config() {
        let dir = temp_dir("cfg");
        let cfg: PomodoroConfig = read_json(&dir, "pomodoro_config.json");
        assert_eq!(cfg.focus_minutes, 25);
        assert_eq!(cfg.short_break_minutes, 5);
        assert_eq!(cfg.long_break_minutes, 15);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tasks_round_trip() {
        let dir = temp_dir("tasks");
        let tasks = vec![FoTask {
            id: "abc".into(),
            task_id: "FO-01".into(),
            title: "Read".into(),
            is_completed: false,
            created_at: "2026-06-03T09:00:00+00:00".into(),
            completed_at: None,
            pomodoro_count: 2,
        }];
        write_json(&dir, "tasks.json", &tasks).unwrap();
        let loaded: Vec<FoTask> = read_json(&dir, "tasks.json");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].task_id, "FO-01");
        assert_eq!(loaded[0].pomodoro_count, 2);
        let _ = fs::remove_dir_all(&dir);
    }
}
