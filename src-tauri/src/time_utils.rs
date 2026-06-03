use chrono::Local;

/// Today's date as "YYYY-MM-DD" (local time).
pub fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Current local time as an RFC3339 timestamp.
pub fn now_rfc3339() -> String {
    Local::now().to_rfc3339()
}
