//! Jarvis-style host grounding: classify the user turn and fetch evidence
//! *before* the model answers.
//!
//! OpenHuman's default loop asks the LLM to call `memory_recall` /
//! `web_search_tool`. Cheap chat models skip those tools and answer from
//! weights ("I don't know you", "SpaceX is private", "search isn't available").
//! Jarvis never left that to the model: it classified intent, ran search or
//! vault recall, and injected the results into the prompt.
//!
//! This module is that same contract, without a second LLM classify call.

use crate::openhuman::config::Config;
use crate::openhuman::memory::store::chunks::store::with_connection;
use serde_json::json;
use std::path::Path;

/// What the host should fetch before the model sees the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundingKind {
    /// Identity / vault questions ("what do you know about me").
    Recall,
    /// Live / current facts (weather, prices, news, "search it").
    LiveSearch,
}

/// Deterministic intent gate. Keep this conservative: a wrong LiveSearch
/// costs one search call; a missed Recall is the "I don't know you" bug.
pub fn classify_grounding(text: &str) -> Option<GroundingKind> {
    let t = text.trim().to_ascii_lowercase();
    if t.is_empty() {
        return None;
    }
    if is_live_search(&t) {
        return Some(GroundingKind::LiveSearch);
    }
    if is_recall(&t) {
        return Some(GroundingKind::Recall);
    }
    None
}

fn is_live_search(t: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "weather",
        "forecast",
        "temperature",
        "stock price",
        "share price",
        "ticker",
        "ipo",
        "publicly traded",
        "public company",
        "latest news",
        "current news",
        "who won",
        "search the web",
        "search it",
        "search for",
        "look it up",
        "look that up",
        "google it",
    ];
    if NEEDLES.iter().any(|n| t.contains(n)) {
        return true;
    }
    let wants_now = t.contains("current")
        || t.contains("latest")
        || t.contains("right now")
        || t.contains("today");
    wants_now
        && (t.contains("price")
            || t.contains("news")
            || t.contains("score")
            || t.contains("weather")
            || t.contains("stock"))
}

fn is_recall(t: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "what do you know about me",
        "what do you remember about me",
        "what do you remember",
        "who am i",
        "tell me about myself",
        "tell me about me",
        "what do you know about my",
        "what's my",
        "whats my",
        "do you know me",
        "my background",
        "about my life",
    ];
    NEEDLES.iter().any(|n| t.contains(n))
}

/// Fetch evidence for [`classify_grounding`] and return a prompt block.
/// Failures become an honest block — never "search is unavailable".
///
/// `recent_user_texts` is this thread's prior user turns, oldest first. They
/// carry the subject when the current turn is only a nudge ("yes") or a bare
/// command ("search it and you will find the value").
pub async fn prefetch_grounding(
    config: &Config,
    user_message: &str,
    recent_user_texts: &[&str],
) -> Option<String> {
    let subject = resolve_grounding_query(user_message, recent_user_texts);
    match classify_grounding(subject) {
        Some(GroundingKind::Recall) => Some(prefetch_recall(config, subject)),
        Some(GroundingKind::LiveSearch) => {
            let query = build_search_query(subject, recent_user_texts);
            Some(prefetch_search(config, &query).await)
        }
        None => None,
    }
}

/// Turn a chat turn into a search engine query.
///
/// The raw message is a terrible query: "search it and you will find the value"
/// retrieves price-estimation sites, not the subject the user meant. Strip the
/// imperative wrapper, and when nothing nameable survives, carry the subject
/// forward from the last turn that had one.
pub fn build_search_query(current: &str, recent_user_texts: &[&str]) -> String {
    let stripped = strip_search_command(current);
    if has_named_subject(&stripped) {
        return stripped;
    }

    for text in recent_user_texts.iter().rev() {
        let prior = strip_search_command(text);
        if has_named_subject(&prior) {
            // The subject lives in an earlier turn; the intent ("value",
            // "price") lives in this one. Carry both.
            return match intent_suffix(current) {
                Some(suffix) => format!("{prior} {suffix}"),
                None => prior,
            };
        }
    }

    if stripped.is_empty() {
        current.trim().to_string()
    } else {
        stripped
    }
}

/// Leading imperatives that tell us to search but say nothing about *what*.
/// Longest-first so "search the internet for" wins over "search".
const SEARCH_COMMANDS: &[&str] = &[
    "can you search the internet for",
    "search it on the internet",
    "search on the internet for",
    "search the internet for",
    "search on the internet",
    "search the internet",
    "search the web for",
    "search online for",
    "search the web",
    "search online",
    "look it up online",
    "look at this",
    "search for",
    "search it",
    "look it up",
    "look up",
    "google it",
    "find out",
    "find me",
    "tell me",
    "show me",
    "can you",
    "please",
    // Bare "search" only. A leading bare "google" is more often the subject
    // ("Google stock price") than a command.
    "search",
];

fn strip_search_command(text: &str) -> String {
    let mut out = text.trim().to_string();
    loop {
        let lowered = out.to_ascii_lowercase();
        let Some(hit) = SEARCH_COMMANDS
            .iter()
            .find(|cmd| lowered.starts_with(*cmd))
            .copied()
        else {
            break;
        };
        out = out[hit.len()..]
            .trim_start_matches(|c: char| c.is_whitespace() || c == ',' || c == ':')
            .to_string();
    }
    out.trim().to_string()
}

/// Does the text name something searchable — a proper noun or a ticker?
/// Lowercase filler ("it is not private", "and you will find the value") does
/// not, and must not be sent to a search engine on its own.
fn has_named_subject(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let word = token.trim_matches(|c: char| !c.is_alphanumeric());
        if word.chars().count() < 2 {
            return false;
        }
        if !word.chars().next().is_some_and(|c| c.is_uppercase()) {
            return false;
        }
        !SUBJECT_STOPWORDS.contains(&word.to_ascii_lowercase().as_str())
    })
}

/// Capitalised words that are still not a subject — sentence openers and
/// conversational filler that would otherwise pass [`has_named_subject`].
const SUBJECT_STOPWORDS: &[&str] = &[
    "the", "this", "that", "these", "those", "it", "its", "he", "she", "they", "we", "you", "your",
    "my", "me", "is", "are", "was", "were", "do", "does", "did", "can", "could", "should", "would",
    "will", "what", "whats", "how", "hows", "why", "when", "where", "who", "and", "but", "or",
    "so", "no", "not", "yes", "ok", "okay", "if", "then", "than", "also", "still", "again", "just",
    "now", "today", "tomorrow", "yesterday", "here", "there", "look", "find", "search", "google",
    "tell", "show", "give", "get", "need", "want", "value", "price", "stock", "public", "private",
    "internet", "web", "online", "hey", "hi", "hello", "thanks", "sure", "really", "very",
];

/// What the user wants to know about the carried-forward subject.
fn intent_suffix(text: &str) -> Option<&'static str> {
    let t = text.to_ascii_lowercase();
    if t.contains("stock")
        || t.contains("share price")
        || t.contains("ticker")
        || t.contains("price")
        || t.contains("value")
        || t.contains("quote")
        || t.contains("public")
        || t.contains("private")
        || t.contains("traded")
    {
        return Some("stock price");
    }
    if t.contains("weather") || t.contains("forecast") || t.contains("temperature") {
        return Some("weather");
    }
    if t.contains("news") {
        return Some("latest news");
    }
    None
}

/// Pick the text to ground: this turn, or the latest live-search / recall
/// ask in recent user turns when the current message is only a nudge
/// ("yes", "how long", a frown).
pub fn resolve_grounding_query<'a>(current: &'a str, recent_user_texts: &[&'a str]) -> &'a str {
    if classify_grounding(current).is_some() {
        return current;
    }
    if !is_nudge(current) {
        return current;
    }
    for text in recent_user_texts.iter().rev() {
        if classify_grounding(text).is_some() {
            return text;
        }
    }
    current
}

fn is_nudge(text: &str) -> bool {
    let t = text.trim().to_ascii_lowercase();
    if t.is_empty() {
        return false;
    }
    const EXACT: &[&str] = &[
        "yes", "yep", "yeah", "ok", "okay", "go", "please", "now", "do it",
    ];
    if EXACT.contains(&t.as_str()) {
        return true;
    }
    const PHRASES: &[&str] = &[
        "go ahead",
        "how much time",
        "how long",
        "still waiting",
        "did you check",
        "any update",
        "hello?",
    ];
    if PHRASES.iter().any(|n| t.contains(n)) {
        return true;
    }
    let letters = t.chars().filter(|c| c.is_ascii_alphabetic()).count();
    letters == 0 && t.chars().count() <= 8
}

fn prefetch_recall(config: &Config, user_message: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    let profile = config.workspace_dir.join("PROFILE.md");
    if let Ok(body) = std::fs::read_to_string(&profile) {
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            parts.push(truncate(trimmed, 4000));
        }
    }

    if let Some(vault) = folder_source_root(config) {
        for rel in ["People/Anmol.md", "People/Me.md"] {
            let path = Path::new(vault).join(rel);
            if let Ok(body) = std::fs::read_to_string(&path) {
                let trimmed = body.trim();
                if !trimmed.is_empty() {
                    parts.push(format!("### {rel}\n{}", truncate(trimmed, 4000)));
                    break;
                }
            }
        }
    }

    if let Ok(hits) = keyword_chunks(config, user_message) {
        if !hits.is_empty() {
            parts.push(format!(
                "### Vault notes (keyword)\n{}",
                hits.join("\n\n---\n\n")
            ));
        }
    }

    if parts.is_empty() {
        return "[MEMORY RECALL]\nNo PROFILE.md or vault notes were found. Call `memory_recall` \
                before claiming you do not know the user.\n"
            .to_string();
    }

    format!(
        "[MEMORY RECALL — host-fetched from the user's vault. Answer from this. \
Do not say you have no personal details.]\n\n{}\n",
        parts.join("\n\n")
    )
}

async fn prefetch_search(config: &Config, query: &str) -> String {
    let tools = crate::openhuman::search::build_search_tools(config);
    let Some(tool) = tools.into_iter().find(|t| t.name() == "web_search_tool") else {
        return "[LIVE WEB SEARCH]\nSearch tools are not registered in this config. \
Say that plainly — do not invent weather, prices, or news.\n"
            .to_string();
    };
    match tool.execute(json!({ "query": query.trim() })).await {
        Ok(result) => {
            let body = result.output();
            format!(
                "[LIVE WEB SEARCH RESULTS for \"{}\" — fetched now. These beat training data. \
Answer from them immediately with the fact (temperature, price, headline). \
Where they contradict what you remember, the results are newer and win: say what \
they show rather than repeating what you believe. \
Do not say you will check, do not ask for permission, do not stall. \
Never say search is unavailable.]\n\n{}\n",
                query.trim(),
                truncate(&body, 6000)
            )
        }
        Err(err) => format!(
            "[LIVE WEB SEARCH FAILED]\n{err:#}\n\
The search tool exists; this call failed. Tell the user the lookup failed. \
Do not claim search is unavailable in this chat.\n"
        ),
    }
}

fn folder_source_root(config: &Config) -> Option<&str> {
    use crate::openhuman::memory::sources::types::SourceKind;
    config.memory_sources.iter().find_map(|src| {
        if src.enabled && src.kind == SourceKind::Folder {
            src.path.as_deref()
        } else {
            None
        }
    })
}

fn keyword_chunks(config: &Config, user_message: &str) -> anyhow::Result<Vec<String>> {
    let terms = recall_terms(user_message);
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let cfg = config.clone();
    with_connection(&cfg, move |conn| {
        let mut hits = Vec::new();
        for term in &terms {
            let like = format!("%{term}%");
            let mut stmt = conn.prepare(
                "SELECT substr(content, 1, 500) FROM mem_tree_chunks
                  WHERE content LIKE ?1
                  ORDER BY timestamp_ms DESC
                  LIMIT 3",
            )?;
            let rows = stmt.query_map([&like], |row| row.get::<_, String>(0))?;
            for row in rows {
                let text = row?;
                if !text.trim().is_empty() {
                    hits.push(text);
                }
                if hits.len() >= 6 {
                    return Ok(hits);
                }
            }
        }
        Ok(hits)
    })
}

fn recall_terms(user_message: &str) -> Vec<String> {
    let mut terms = vec![
        "Anmol".to_string(),
        "Boca Raton".to_string(),
        "Florida Atlantic".to_string(),
    ];
    for token in user_message.split_whitespace() {
        let cleaned = token
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_string();
        if cleaned.chars().count() >= 4
            && !["what", "know", "about", "remember", "tell", "yourself"].contains(
                &cleaned.to_ascii_lowercase().as_str(),
            )
        {
            terms.push(cleaned);
        }
    }
    terms.truncate(6);
    terms
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_about_me_as_recall() {
        assert_eq!(
            classify_grounding("what do you know about me"),
            Some(GroundingKind::Recall)
        );
        assert_eq!(
            classify_grounding("Who am I?"),
            Some(GroundingKind::Recall)
        );
    }

    #[test]
    fn classifies_weather_as_live_search() {
        assert_eq!(
            classify_grounding("What is the weather in boca raton"),
            Some(GroundingKind::LiveSearch)
        );
        assert_eq!(
            classify_grounding("So search the weather for boca raton"),
            Some(GroundingKind::LiveSearch)
        );
    }

    #[test]
    fn small_talk_is_not_grounded() {
        assert_eq!(classify_grounding("hey"), None);
        assert_eq!(classify_grounding("thanks"), None);
    }

    #[test]
    fn search_query_drops_the_imperative_wrapper() {
        assert_eq!(build_search_query("Search for Spacex", &[]), "Spacex");
        assert_eq!(
            build_search_query("Google it: Alphabet stock price", &[]),
            "Alphabet stock price"
        );
    }

    #[test]
    fn search_query_carries_the_subject_forward_from_earlier_turns() {
        let prior = [
            "Google stock price",
            "Search for Spacex",
            "Look at this Space Exploration Technologies Corp NASDAQ: SPCX",
        ];
        // "and you will find the value" names nothing — searching it verbatim
        // is what returned unrelated price-estimation sites.
        assert_eq!(
            build_search_query("Search it and you will find the value", &prior),
            "Space Exploration Technologies Corp NASDAQ: SPCX stock price"
        );
        assert_eq!(
            build_search_query("Search on the internet it is not private", &prior),
            "Space Exploration Technologies Corp NASDAQ: SPCX stock price"
        );
    }

    #[test]
    fn search_query_keeps_a_subjectless_question_intact() {
        // Weather has no proper noun and no prior subject — the raw ask is
        // already the best query.
        assert_eq!(
            build_search_query("How's the weather today", &[]),
            "How's the weather today"
        );
    }

    #[test]
    fn nudge_reuses_prior_weather_ask() {
        let prior = ["How's the weather today"];
        assert_eq!(
            resolve_grounding_query("Yes", &prior),
            "How's the weather today"
        );
        assert_eq!(
            resolve_grounding_query("How much time do you need", &prior),
            "How's the weather today"
        );
        assert_eq!(resolve_grounding_query("☹️", &prior), "How's the weather today");
        assert_eq!(resolve_grounding_query("hey", &prior), "hey");
    }
}
