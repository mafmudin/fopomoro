use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FoTask {
    pub id: String,                  // UUID string
    pub task_id: String,             // display id, e.g. "FO-01"
    pub title: String,
    pub is_completed: bool,
    pub created_at: String,          // RFC3339
    pub completed_at: Option<String>,
    pub pomodoro_count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FoSession {
    pub date: String,                // "YYYY-MM-DD"
    pub focus_sessions_count: i32,
    pub total_minutes_studied: i32,
    pub tasks_completed_count: i32,
}

impl Default for FoSession {
    fn default() -> Self {
        Self {
            date: crate::time_utils::today_string(),
            focus_sessions_count: 0,
            total_minutes_studied: 0,
            tasks_completed_count: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PomodoroConfig {
    pub focus_minutes: i32,
    pub short_break_minutes: i32,
    pub long_break_minutes: i32,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self { focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowSettings {
    pub opacity: f64,
    // `serde(default)` keeps older settings.json (opacity-only) readable so the
    // saved opacity isn't lost when upgrading to a build that has bg_color.
    #[serde(default = "default_bg_color")]
    pub bg_color: String,
}

fn default_bg_color() -> String {
    "#1E1E2E".to_string()
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self { opacity: 0.9, bg_color: default_bg_color() }
    }
}
