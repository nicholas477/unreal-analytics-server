pub fn session_duration_to_string(session_duration: &chrono::TimeDelta) -> String {
    format!(
        "{}:{:0>2}:{:0>2}",
        session_duration.num_hours(),
        session_duration.num_minutes() % 60,
        session_duration.num_seconds() % 60
    )
}
