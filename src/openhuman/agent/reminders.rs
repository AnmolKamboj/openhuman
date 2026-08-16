//! Host-side reminder capture: "remind me to X in 5 minutes" becomes a cron
//! job before the model ever sees the turn.
//!
//! Channel turns hand the provider ~370 tools, far past the 128-function cap,
//! so the harness falls back to prompt-guided tools and the model reliably
//! answers "sure, I'll remind you" without emitting a `schedule` call. Even
//! when it does call one, `schedule` is an external-effect tool and a channel
//! turn is untrusted, so the approval gate parks it with nowhere to route the
//! approval. A reminder the user typed themselves needs neither guess.

use chrono::{DateTime, Duration, Utc};

/// A reminder the user asked for in this turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderRequest {
    /// What to be reminded about, as the user phrased it ("do laundry").
    pub task: String,
    /// When it fires.
    pub at: DateTime<Utc>,
}

/// Parse "remind me to do laundry in 5 minutes" and its common phrasings.
///
/// Deliberately narrow: only an explicit reminder verb plus a relative delay.
/// Anything else returns `None` and takes the normal model path, so this never
/// hijacks a turn it doesn't fully understand.
pub fn parse_reminder(text: &str, now: DateTime<Utc>) -> Option<ReminderRequest> {
    let lowered = text.to_ascii_lowercase();
    if !mentions_reminder(&lowered) {
        return None;
    }
    let (delay, delay_span) = parse_relative_delay(&lowered)?;
    let task = extract_task(text, &lowered, delay_span)?;
    Some(ReminderRequest {
        task,
        at: now + delay,
    })
}

fn mentions_reminder(lowered: &str) -> bool {
    const VERBS: &[&str] = &["remind me", "reminder", "remind us"];
    VERBS.iter().any(|verb| lowered.contains(verb))
}

/// Find "in 5 minutes" / "in 2 hrs" and return the delay plus the byte range
/// it occupied, so the task text can be cut around it.
fn parse_relative_delay(lowered: &str) -> Option<(Duration, (usize, usize))> {
    let mut search_from = 0;
    while let Some(rel) = lowered[search_from..].find("in ") {
        let start = search_from + rel;
        let rest = &lowered[start + 3..];
        let mut chars = rest.char_indices();
        let mut digits = String::new();
        let mut cursor = 0;
        for (idx, ch) in chars.by_ref() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                cursor = idx + ch.len_utf8();
            } else {
                cursor = idx;
                break;
            }
        }
        if digits.is_empty() {
            search_from = start + 3;
            continue;
        }
        let unit_part = rest[cursor..].trim_start();
        let unit_offset = rest[cursor..].len() - unit_part.len();
        let unit_word: String = unit_part
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let Some(duration) = unit_duration(&unit_word, digits.parse::<i64>().ok()?) else {
            search_from = start + 3;
            continue;
        };
        let end = start + 3 + cursor + unit_offset + unit_word.len();
        return Some((duration, (start, end)));
    }
    None
}

fn unit_duration(unit: &str, amount: i64) -> Option<Duration> {
    if amount <= 0 {
        return None;
    }
    match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => Some(Duration::seconds(amount)),
        "m" | "min" | "mins" | "minute" | "minutes" => Some(Duration::minutes(amount)),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(Duration::hours(amount)),
        "d" | "day" | "days" => Some(Duration::days(amount)),
        _ => None,
    }
}

/// The task is whatever sits between the reminder verb and the delay, minus
/// the connective words around it ("to", "about", "that i should").
fn extract_task(original: &str, lowered: &str, delay_span: (usize, usize)) -> Option<String> {
    let (delay_start, delay_end) = delay_span;
    let body_start = reminder_body_start(lowered)?;
    let mut task = if delay_start > body_start {
        original[body_start..delay_start].to_string()
    } else {
        // "remind me in 5 minutes to do laundry" — the task trails the delay.
        original[delay_end..].to_string()
    };
    task = task
        .trim()
        .trim_start_matches(|c: char| c == ',' || c == ':' || c == '-')
        .trim()
        .to_string();
    for prefix in ["to ", "that ", "about ", "for "] {
        if task.to_ascii_lowercase().starts_with(prefix) {
            task = task[prefix.len()..].trim().to_string();
        }
    }
    let task = task
        .trim_end_matches(|c: char| c == '.' || c == '!' || c == ',')
        .trim()
        .to_string();
    if task.is_empty() {
        None
    } else {
        Some(task)
    }
}

fn reminder_body_start(lowered: &str) -> Option<usize> {
    const VERBS: &[&str] = &["remind me", "remind us", "reminder"];
    VERBS
        .iter()
        .filter_map(|verb| lowered.find(verb).map(|idx| idx + verb.len()))
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Option<ReminderRequest> {
        let now = DateTime::parse_from_rfc3339("2026-08-15T21:33:00Z")
            .unwrap()
            .with_timezone(&Utc);
        parse_reminder(text, now)
    }

    #[test]
    fn captures_the_task_and_the_delay() {
        let parsed = at("Set a reminder for me to do laundry in 5 minutes").unwrap();
        assert_eq!(parsed.task, "do laundry");
        assert_eq!(parsed.at.to_rfc3339(), "2026-08-15T21:38:00+00:00");
    }

    #[test]
    fn reads_the_task_after_the_delay_too() {
        let parsed = at("remind me in 2 hours to call mom").unwrap();
        assert_eq!(parsed.task, "call mom");
        assert_eq!(parsed.at.to_rfc3339(), "2026-08-15T23:33:00+00:00");
    }

    #[test]
    fn accepts_short_unit_spellings() {
        assert_eq!(at("remind me to stretch in 30 mins").unwrap().task, "stretch");
        assert_eq!(at("remind me to sleep in 1 hr").unwrap().task, "sleep");
        assert_eq!(at("remind me to stand in 45 s").unwrap().task, "stand");
    }

    #[test]
    fn ignores_turns_without_a_delay_or_a_task() {
        assert!(at("remind me about the thing").is_none());
        assert!(at("remind me in 5 minutes").is_none());
        assert!(at("what did you remind me of").is_none());
    }

    #[test]
    fn is_not_fooled_by_an_unrelated_in_phrase() {
        // "in the kitchen" is not a delay; the real one comes later.
        let parsed = at("remind me to clean in the kitchen in 10 minutes").unwrap();
        assert_eq!(parsed.task, "clean in the kitchen");
    }

    #[test]
    fn leaves_non_reminder_turns_alone() {
        assert!(at("what's the weather in 5 minutes").is_none());
        assert!(at("set a timer for pasta").is_none());
    }
}
