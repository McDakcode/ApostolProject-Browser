//! apb-ai
//!
//! Local-first AI subsystem (design doc §14) with a hard Privacy Firewall
//! (§10A): before *any* byte leaves the machine towards a cloud provider,
//! every message and every piece of injected context is passed through the
//! `SecretScanner`. Secrets are replaced with markers and never sent.
//! Cloud providers are opt-in; the default configuration points at a local
//! Ollama endpoint so nothing leaves the device unless explicitly asked.
//!
//! The assistant can propose side effects through fenced `apb-action`
//! blocks; destructive/stateful actions carry `requires_confirmation() ==
//! true` and the UI must ask before executing them.

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("http transport error: {0}")]
    Http(String),
    #[error("provider returned an unreadable response: {0}")]
    BadResponse(String),
    #[error("API key env var '{0}' is not set")]
    ApiKeyMissing(String),
}

pub type Result<T> = std::result::Result<T, AiError>;

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Any OpenAI-compatible /v1/chat/completions endpoint.
    OpenAiCompatible,
    AnthropicCompatible,
    Ollama,
    CustomHttp,
}

impl ProviderKind {
    /// True when requests stay on this machine (no network egress).
    pub fn is_local(&self) -> bool {
        matches!(self, ProviderKind::Ollama)
    }
}

/// Connection settings for one AI provider. Note there is deliberately no
/// `api_key` field: keys live in environment variables and are resolved at
/// call time, so they never touch disk or config files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    /// Full endpoint URL (e.g. `http://127.0.0.1:11434/v1/chat/completions`).
    pub base_url: String,
    pub model: String,
    /// Env var *name* holding the key for cloud providers.
    pub api_key_env: Option<String>,
    /// Hard cap on how much profile context may be attached (chars).
    pub max_context_chars: usize,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: ProviderKind::Ollama,
            base_url: "http://127.0.0.1:11434/v1/chat/completions".to_string(),
            model: "llama3.2".to_string(),
            api_key_env: None,
            max_context_chars: 6000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }
}

const SYSTEM_PREAMBLE: &str = "\
You are APB's built-in browsing assistant running inside the APB browser. \
Respect the user's privacy: never repeat sensitive data back verbatim. \
When you want to change browser state, emit an ```apb-action fenced block \
with one action per line.";

// ---------------------------------------------------------------------------
// Privacy Firewall — secret scanning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    AwsAccessKey,
    PrivateKey,
    DatabaseUrl,
    SessionCookie,
    PasswordField,
    BearerToken,
    ApiToken,
    CreditCard,
}

impl SecretKind {
    pub fn label(&self) -> &'static str {
        match self {
            SecretKind::AwsAccessKey => "AWS access key",
            SecretKind::PrivateKey => "private key",
            SecretKind::DatabaseUrl => "database URL",
            SecretKind::SessionCookie => "session cookie",
            SecretKind::PasswordField => "password",
            SecretKind::BearerToken => "bearer token",
            SecretKind::ApiToken => "API token",
            SecretKind::CreditCard => "card number",
        }
    }

    /// Marker substituted into outbound text. DatabaseUrl keeps the `@` so
    /// hosts remain readable in logs after scrubbing.
    fn redaction_marker(&self) -> &'static str {
        match self {
            SecretKind::AwsAccessKey => "[REDACTED:aws-access-key]",
            SecretKind::PrivateKey => "[REDACTED:private-key]",
            SecretKind::DatabaseUrl => "[REDACTED:database-credentials]@",
            SecretKind::SessionCookie => "[REDACTED:cookie]",
            SecretKind::PasswordField => "[REDACTED:password]",
            SecretKind::BearerToken => "[REDACTED:bearer-token]",
            SecretKind::ApiToken => "[REDACTED:api-token]",
            SecretKind::CreditCard => "[REDACTED:card-number]",
        }
    }
}

/// A found secret: byte offsets into the scanned string (always on UTF-8
/// char boundaries because every matcher is ASCII-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub kind: SecretKind,
    pub start: usize,
    pub end: usize,
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Case-insensitive search for `needle` at word boundaries.
/// Returns byte offsets of the match start.
fn find_all_ci(haystack: &str, needle: &str) -> Vec<usize> {
    find_all_ci_inner(haystack, needle, true)
}

/// Same, but only requires a boundary *before* the match — for token
/// prefixes (`AKIA`, `ghp_`, …) that sit inside longer secret bodies.
fn find_all_ci_open_ended(haystack: &str, needle: &str) -> Vec<usize> {
    find_all_ci_inner(haystack, needle, false)
}

fn find_all_ci_inner(haystack: &str, needle: &str, require_next_boundary: bool) -> Vec<usize> {
    assert!(needle.is_ascii());
    let hay = haystack.as_bytes();
    let needle_lc = needle.to_ascii_lowercase();
    let hay_lc: Vec<u8> = hay.iter().map(|b| b.to_ascii_lowercase()).collect();
    let mut out = Vec::new();
    if needle_lc.is_empty() || hay_lc.len() < needle_lc.len() {
        return out;
    }
    let mut i = 0;
    while i + needle_lc.len() <= hay_lc.len() {
        if &hay_lc[i..i + needle_lc.len()] == needle_lc.as_bytes() {
            let prev_ok = i == 0 || !is_word_byte(hay[i - 1]);
            let next_ok = !require_next_boundary
                || i + needle_lc.len() == hay.len()
                || !is_word_byte(hay[i + needle_lc.len()]);
            if prev_ok && next_ok {
                out.push(i);
                i += needle_lc.len();
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Run of non-whitespace bytes starting at `start`, with common trailing
/// punctuation trimmed (so `password=abc123;` does not keep the `;`).
fn value_run(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut end = start;
    while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
        end += 1;
    }
    while end > start && matches!(bytes[end - 1], b'.' | b',' | b';' | b')' | b'"' | b'\'' | b']') {
        end -= 1;
    }
    end
}

fn luhn_valid(digits: &[u8]) -> bool {
    let mut sum = 0u32;
    let mut double = false;
    for &d in digits.iter().rev() {
        let mut n = (d as char).to_digit(10).expect("digit");
        if double {
            n *= 2;
            if n > 9 {
                n -= 9;
            }
        }
        sum += n;
        double = !double;
    }
    sum % 10 == 0
}

/// Stateless scanner: pure function over text, safe to call from anywhere.
pub struct SecretScanner;

impl SecretScanner {
    pub fn scan(text: &str) -> Vec<Detection> {
        let mut out = Vec::new();
        scan_pem(text, &mut out);
        scan_database_urls(text, &mut out);
        scan_cookie_headers(text, &mut out);
        scan_keyword_assignments(text, &mut out);
        scan_bearer_tokens(text, &mut out);
        scan_vendor_tokens(text, &mut out);
        scan_aws_keys(text, &mut out);
        scan_credit_cards(text, &mut out);

        // Order by position; drop overlaps keeping the earliest detector.
        out.sort_by_key(|d| (d.start, d.end));
        let mut kept: Vec<Detection> = Vec::new();
        for d in out {
            match kept.last() {
                Some(prev) if d.start < prev.end => continue,
                _ => kept.push(d),
            }
        }
        kept
    }

    /// Convenience: returns scrubbed text plus what was removed.
    pub fn redact(text: &str) -> (String, Vec<Detection>) {
        let detections = Self::scan(text);
        let mut out = String::with_capacity(text.len());
        let mut pos = 0usize;
        for d in &detections {
            out.push_str(&text[pos..d.start]);
            out.push_str(d.kind.redaction_marker());
            pos = d.end;
        }
        out.push_str(&text[pos..]);
        (out, detections)
    }
}

fn scan_pem(text: &str, out: &mut Vec<Detection>) {
    let mut offset = 0usize;
    let mut open: Option<(usize, bool)> = None; // (start, is_private)
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("-----BEGIN") && trimmed.contains("PRIVATE KEY") {
            open = Some((offset, true));
        } else if let Some((start, true)) = open {
            if trimmed.starts_with("-----END") {
                out.push(Detection {
                    kind: SecretKind::PrivateKey,
                    start,
                    end: offset + line.len(),
                });
                open = None;
            }
        }
        offset += line.len();
    }
    // Unterminated block: still treat everything from BEGIN as secret.
    if let Some((start, true)) = open {
        out.push(Detection { kind: SecretKind::PrivateKey, start, end: text.len() });
    }
}

fn scan_database_urls(text: &str, out: &mut Vec<Detection>) {
    const SCHEMES: &[&str] = &[
        "postgres", "postgresql", "mysql", "mariadb", "mongodb", "mongodb+srv",
        "redis", "rediss", "amqp", "mssql", "clickhouse",
    ];
    for (colon_pos, matched) in text.match_indices("://") {
        // Scheme = trailing word-chars before "://".
        let mut s = colon_pos;
        while s > 0 && is_word_byte(text.as_bytes()[s - 1]) {
            s -= 1;
        }
        let scheme = &text[s..colon_pos];
        if !SCHEMES.contains(&scheme.to_ascii_lowercase().as_str()) {
            continue;
        }
        // Authority ends at whitespace, quote, ')' or first '/'.
        let auth_start = colon_pos + matched.len();
        let rest = &text[auth_start..];
        let mut end = auth_start;
        for (i, c) in rest.char_indices() {
            if c.is_whitespace() || c == '"' || c == '\'' || c == ')' || c == '/' {
                break;
            }
            end = auth_start + i + c.len_utf8();
        }
        let authority = &text[auth_start..end];
        if let Some(at) = authority.rfind('@') {
            if authority[..at].contains(':') {
                out.push(Detection {
                    kind: SecretKind::DatabaseUrl,
                    start: auth_start,
                    end: auth_start + at, // exclude '@'; marker re-adds it
                });
            }
        }
    }
}

fn scan_cookie_headers(text: &str, out: &mut Vec<Detection>) {
    for pos in find_all_ci(text, "cookie") {
        let bytes = text.as_bytes();
        let mut p = pos + "cookie".len();
        while p < bytes.len() && bytes[p] == b' ' {
            p += 1;
        }
        if p >= bytes.len() || bytes[p] != b':' {
            continue;
        }
        p += 1;
        while p < bytes.len() && bytes[p] == b' ' {
            p += 1;
        }
        let end = text[p..].find('\n').map(|n| p + n).unwrap_or(text.len());
        let end = value_run(&text[..end], p).max(p);
        if end.saturating_sub(p) >= 10 {
            out.push(Detection { kind: SecretKind::SessionCookie, start: p, end });
        }
    }
}

fn scan_keyword_assignments(text: &str, out: &mut Vec<Detection>) {
    const PASSWORD_WORDS: &[&str] = &["password", "passwd", "pwd"];
    const SESSION_WORDS: &[&str] =
        &["sessionid", "session_id", "sess_id", "authenticity_token", "auth_token"];

    let push_value = |text: &str, after: usize, kind: SecretKind, min_len: usize, out: &mut Vec<Detection>| {
        let bytes = text.as_bytes();
        let mut p = after;
        while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
            p += 1;
        }
        if p < bytes.len() && bytes[p] == b'=' {
            p += 1;
            while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                p += 1;
            }
            let end = value_run(text, p);
            if end >= p && end.saturating_sub(p) >= min_len {
                out.push(Detection { kind, start: p, end });
            }
        }
    };

    for w in PASSWORD_WORDS {
        for pos in find_all_ci(text, w) {
            push_value(text, pos + w.len(), SecretKind::PasswordField, 6, out);
        }
    }
    for w in SESSION_WORDS {
        for pos in find_all_ci(text, w) {
            push_value(text, pos + w.len(), SecretKind::SessionCookie, 8, out);
        }
    }
}

fn scan_bearer_tokens(text: &str, out: &mut Vec<Detection>) {
    for pos in find_all_ci(text, "bearer") {
        let bytes = text.as_bytes();
        let mut p = pos + "bearer".len();
        while p < bytes.len() && bytes[p] == b' ' {
            p += 1;
        }
        let mut end = p;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric()
                || matches!(bytes[end], b'.' | b'_' | b'~' | b'+' | b'/' | b'=' | b'-'))
        {
            end += 1;
        }
        if end.saturating_sub(p) >= 16 {
            out.push(Detection { kind: SecretKind::BearerToken, start: pos, end });
        }
    }
}

const VENDOR_PREFIXES: &[&str] = &[
    "ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_", "glpat-",
    "xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-",
    "sk-ant-", "sk-proj-", "sk-live-", "sk-test-",
];

fn scan_vendor_tokens(text: &str, out: &mut Vec<Detection>) {
    let bytes = text.as_bytes();
    for prefix in VENDOR_PREFIXES {
        for pos in find_all_ci_open_ended(text, prefix) {
            let mut end = pos + prefix.len();
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'-' | b'_'))
            {
                end += 1;
            }
            if end.saturating_sub(pos) >= 20 {
                out.push(Detection { kind: SecretKind::ApiToken, start: pos, end });
            }
        }
    }
    // Google-style keys: AIza + 30+ url-safe chars.
    for pos in find_all_ci_open_ended(text, "AIza") {
        let mut end = pos + 4;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
        {
            end += 1;
        }
        if end.saturating_sub(pos) >= 35 {
            out.push(Detection { kind: SecretKind::ApiToken, start: pos, end });
        }
    }
}

fn scan_aws_keys(text: &str, out: &mut Vec<Detection>) {
    for pos in find_all_ci_open_ended(text, "AKIA") {
        let bytes = text.as_bytes();
        if pos + 20 > bytes.len() {
            continue;
        }
        let body = &bytes[pos + 4..pos + 20];
        if !body.iter().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
            continue;
        }
        // Guard against being the head of an even longer run.
        if bytes.len() > pos + 20 && is_word_byte(bytes[pos + 20]) {
            continue;
        }
        out.push(Detection { kind: SecretKind::AwsAccessKey, start: pos, end: pos + 20 });
    }
}

fn scan_credit_cards(text: &str, out: &mut Vec<Detection>) {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            // Extend over digits with single spaces/dashes between groups.
            let mut end = i;
            let mut digits: Vec<u8> = Vec::new();
            let mut last_sep = false;
            while end < bytes.len() {
                let b = bytes[end];
                if b.is_ascii_digit() {
                    digits.push(b);
                    last_sep = false;
                    end += 1;
                } else if (b == b' ' || b == b'-')
                    && !last_sep
                    && end + 1 < bytes.len()
                    && bytes[end + 1].is_ascii_digit()
                {
                    last_sep = true;
                    end += 1;
                } else {
                    break;
                }
            }
            if (13..=19).contains(&digits.len()) && luhn_valid(&digits) {
                out.push(Detection { kind: SecretKind::CreditCard, start: i, end });
                i = end;
                continue;
            }
            i += 1.max(digits.len());
        } else {
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Context assembly (§14.5 — explicit permissions, minimal surface)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    Page,
    Tabs,
    History,
    Notes,
    Clipboard,
}

#[derive(Debug, Clone)]
pub struct ContextPiece {
    pub source: ContextSource,
    pub title: String,
    pub body: String,
}

/// What the user allowed the assistant to see. Everything defaults to the
/// minimum; the UI exposes these as toggles per conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPermissions {
    pub page_content: bool,
    pub open_tabs: bool,
    pub history: bool,
    pub notes: bool,
    pub clipboard: bool,
    /// Run the Privacy Firewall over assembled context before sending.
    pub strip_secrets: bool,
    pub max_chars: usize,
}

impl Default for ContextPermissions {
    fn default() -> Self {
        Self {
            page_content: true,
            open_tabs: false,
            history: false,
            notes: false,
            clipboard: false,
            strip_secrets: true,
            max_chars: 6000,
        }
    }
}

impl ContextPermissions {
    fn allows(&self, source: ContextSource) -> bool {
        match source {
            ContextSource::Page => self.page_content,
            ContextSource::Tabs => self.open_tabs,
            ContextSource::History => self.history,
            ContextSource::Notes => self.notes,
            ContextSource::Clipboard => self.clipboard,
        }
    }
}

/// Assemble permitted pieces into one context string, capped at
/// `max_chars` (cut on a char boundary), optionally scrubbed of secrets.
/// Returns the final context plus how many secrets were stripped.
pub fn build_context(perms: &ContextPermissions, pieces: &[ContextPiece]) -> (String, usize) {
    let mut buf = String::new();
    let mut stripped = 0usize;
    for piece in pieces {
        if !perms.allows(piece.source) || buf.len() >= perms.max_chars {
            continue;
        }
        let section = format!("--- {} ---\n{}\n\n", piece.title, piece.body);
        let remaining = perms.max_chars - buf.len();
        let section = truncate_chars(&section, remaining);
        if perms.strip_secrets {
            let (clean, dets) = SecretScanner::redact(&section);
            stripped += dets.len();
            buf.push_str(&clean);
        } else {
            buf.push_str(&section);
        }
    }
    (buf.trim_end().to_string(), stripped)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    match s.char_indices().nth(max) {
        Some((idx, _)) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Assistant-initiated browser actions (§14.7)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiAction {
    SummarizePage,
    ExtractActions,
    OrganizeTabs,
    CreateNote { title: String },
    OpenUrl { url: String },
    InsertText { text: String },
}

impl AiAction {
    /// State-changing actions need an explicit user confirmation click.
    pub fn requires_confirmation(&self) -> bool {
        matches!(
            self,
            AiAction::CreateNote { .. } | AiAction::OpenUrl { .. } | AiAction::InsertText { .. }
        )
    }

    pub fn describe(&self) -> String {
        match self {
            AiAction::SummarizePage => "Summarize the current page".into(),
            AiAction::ExtractActions => "Extract action items from the page".into(),
            AiAction::OrganizeTabs => "Organize open tabs into groups".into(),
            AiAction::CreateNote { title } => format!("Create note \"{title}\""),
            AiAction::OpenUrl { url } => format!("Open {url}"),
            AiAction::InsertText { .. } => "Insert generated text at cursor".into(),
        }
    }

    fn parse_line(line: &str) -> Option<Self> {
        let line = line.trim().trim_start_matches('-').trim();
        let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
        let cmd = cmd.trim().to_ascii_lowercase();
        let arg = arg.trim().trim_matches('"').to_string();
        match cmd.as_str() {
            "summarize-page" => Some(AiAction::SummarizePage),
            "extract-actions" => Some(AiAction::ExtractActions),
            "organize-tabs" => Some(AiAction::OrganizeTabs),
            "create-note" if !arg.is_empty() => Some(AiAction::CreateNote { title: arg }),
            "open-url" if !arg.is_empty() => Some(AiAction::OpenUrl { url: arg }),
            "insert-text" if !arg.is_empty() => Some(AiAction::InsertText { text: arg }),
            _ => None,
        }
    }
}

/// Extract ```apb-action fenced blocks from an assistant reply.
pub fn parse_actions(reply: &str) -> Vec<AiAction> {
    let mut actions = Vec::new();
    let mut rest = reply;
    while let Some(start) = rest.find("```apb-action") {
        let after = &rest[start + "```apb-action".len()..];
        let Some(end) = after.find("```") else { break };
        for line in after[..end].lines() {
            if let Some(a) = AiAction::parse_line(line) {
                actions.push(a);
            }
        }
        rest = &after[end + 3..];
    }
    actions
}

// ---------------------------------------------------------------------------
// Transport + client
// ---------------------------------------------------------------------------

/// Outbound HTTP seam so tests can stub providers without network access.
pub trait HttpTransport: Send + Sync {
    fn post_json(&self, url: &str, headers: &[(String, String)], body: &str) -> Result<String>;
}

/// Real transport backed by ureq (native-tls), 60s timeout.
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    pub fn new() -> Self {
        Self { agent: ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(60)).build() }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport for UreqTransport {
    fn post_json(&self, url: &str, headers: &[(String, String)], body: &str) -> Result<String> {
        let mut req = self.agent.post(url).set("Content-Type", "application/json");
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let resp = req.send_string(body).map_err(|e| AiError::Http(e.to_string()))?;
        resp.into_string().map_err(|e| AiError::Http(e.to_string()))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatReport {
    pub reply: String,
    pub actions: Vec<AiAction>,
    pub secrets_blocked: usize,
    pub provider_local: bool,
}

pub struct AiClient<T: HttpTransport> {
    config: ProviderConfig,
    transport: T,
}

impl<T: HttpTransport> AiClient<T> {
    pub fn new(config: ProviderConfig, transport: T) -> Self {
        Self { config, transport }
    }

    fn resolve_api_key(&self) -> Result<Option<String>> {
        match self.config.kind {
            ProviderKind::Ollama => Ok(None),
            _ => match &self.config.api_key_env {
                Some(var) => std::env::var(var)
                    .map(Some)
                    .map_err(|_| AiError::ApiKeyMissing(var.clone())),
                None => Err(AiError::ApiKeyMissing("(unconfigured)".to_string())),
            },
        }
    }

    /// One conversational turn. `context` should come from `build_context`.
    /// Firewall runs regardless of provider locality — defense in depth.
    pub fn chat(&self, messages: &[ChatMessage], context: &str) -> Result<ChatReport> {
        let mut blocked = 0usize;

        // 1. Firewall the outbound content.
        let mut scrubbed: Vec<ChatMessage> = Vec::with_capacity(messages.len());
        for m in messages {
            let (clean, dets) = SecretScanner::redact(&m.content);
            blocked += dets.len();
            scrubbed.push(ChatMessage { role: m.role, content: clean });
        }
        let (ctx_clean, ctx_dets) = SecretScanner::redact(context);
        blocked += ctx_dets.len();

        // 2. Compose payloads per provider family.
        let system_text = format!("{SYSTEM_PREAMBLE}\n\n--- Context ---\n{ctx_clean}");
        let (url, body) = match self.config.kind {
            ProviderKind::AnthropicCompatible => {
                let msgs: Vec<serde_json::Value> = scrubbed
                    .iter()
                    .filter(|m| m.role != Role::System)
                    .map(|m| {
                        serde_json::json!({
                            "role": if m.role == Role::Assistant { "assistant" } else { "user" },
                            "content": m.content,
                        })
                    })
                    .collect();
                (
                    self.config.base_url.clone(),
                    serde_json::json!({
                        "model": self.config.model,
                        "max_tokens": 2048,
                        "system": system_text,
                        "messages": msgs,
                        "stream": false,
                    })
                    .to_string(),
                )
            }
            _ => {
                let msgs: Vec<serde_json::Value> = std::iter::once(serde_json::json!({
                    "role": "system", "content": system_text
                }))
                .chain(scrubbed.iter().map(|m| {
                    serde_json::json!({
                        "role": match m.role {
                            Role::System => "system",
                            Role::User => "user",
                            Role::Assistant => "assistant",
                        },
                        "content": m.content,
                    })
                }))
                .collect();
                (
                    self.config.base_url.clone(),
                    serde_json::json!({
                        "model": self.config.model,
                        "messages": msgs,
                        "stream": false,
                    })
                    .to_string(),
                )
            }
        };

        // 3. Headers / auth.
        let mut headers: Vec<(String, String)> = Vec::new();
        if let Some(key) = self.resolve_api_key()? {
            match self.config.kind {
                ProviderKind::AnthropicCompatible => {
                    headers.push(("x-api-key".into(), key));
                    headers.push(("anthropic-version".into(), "2023-06-01".into()));
                }
                _ => headers.push(("Authorization".into(), format!("Bearer {key}"))),
            }
        }

        // 4. Send.
        let raw = self.transport.post_json(&url, &headers, &body)?;

        // 5. Parse reply across provider shapes.
        let v: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| AiError::BadResponse(e.to_string()))?;
        let reply = extract_reply_text(&v)
            .ok_or_else(|| AiError::BadResponse("no message content found".into()))?;

        // 6. Firewall the inbound direction too (models echo prompts back).
        let (reply_clean, reply_dets) = SecretScanner::redact(&reply);
        blocked += reply_dets.len();

        Ok(ChatReport {
            actions: parse_actions(&reply_clean),
            secrets_blocked: blocked,
            provider_local: self.config.kind.is_local(),
            reply: reply_clean,
        })
    }
}

fn extract_reply_text(v: &serde_json::Value) -> Option<String> {
    // OpenAI shape: choices[0].message.content
    if let Some(c) = v["choices"][0]["message"]["content"].as_str() {
        return Some(c.to_string());
    }
    // Anthropic shape: content[0].text
    if let Some(t) = v["content"][0]["text"].as_str() {
        return Some(t.to_string());
    }
    // Ollama native shape: message.content
    if let Some(c) = v["message"]["content"].as_str() {
        return Some(c.to_string());
    }
    None
}

// ---------------------------------------------------------------------------
// Tests (offline — MockTransport stands in for the provider)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn aws_key_and_password_are_the_only_two_detections() {
        let (clean, dets) =
            SecretScanner::redact("key AKIAIOSFODNN7EXAMPLE and password=hunter2secret");
        assert_eq!(dets.len(), 2, "got: {dets:?}");
        assert!(!clean.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!clean.contains("hunter2secret"));
        assert_eq!(dets[0].kind, SecretKind::AwsAccessKey);
        assert_eq!(dets[1].kind, SecretKind::PasswordField);
    }

    #[test]
    fn pem_block_is_caught_across_lines() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQ\nabc==\n-----END RSA PRIVATE KEY-----\n";
        let (clean, dets) = SecretScanner::redact(pem);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].kind, SecretKind::PrivateKey);
        assert!(!clean.contains("MIIEow"));
    }

    #[test]
    fn database_url_creds_removed_but_host_kept() {
        let (clean, dets) =
            SecretScanner::redact("connect via postgresql://admin:s3cretPw@db.internal:5432/prod today");
        assert_eq!(dets[0].kind, SecretKind::DatabaseUrl);
        assert!(!clean.contains("s3cretPw"));
        assert!(!clean.contains("admin:"));
        assert!(clean.contains("db.internal:5432/prod"));
    }

    #[test]
    fn cookies_headers_and_bearer_tokens_flagged() {
        let text = "Cookie: sessionid=abcdef123456789; theme=dark\nAuthorization: Bearer abcdefghijklmnopqrstuvwx\n";
        let (_, dets) = SecretScanner::redact(text);
        let kinds: Vec<_> = dets.iter().map(|d| d.kind).collect();
        assert!(kinds.contains(&SecretKind::SessionCookie), "{kinds:?}");
        assert!(kinds.contains(&SecretKind::BearerToken), "{kinds:?}");
    }

    #[test]
    fn luhn_cards_pass_fail_correctly() {
        let (clean, dets) = SecretScanner::redact("pay with 4242 4242 4242 4242 please");
        assert_eq!(dets[0].kind, SecretKind::CreditCard);
        assert!(!clean.contains("4242"));
        // Fails Luhn -> untouched.
        let (_, dets) = SecretScanner::redact("invoice 1234 5678 9012 3456 due");
        assert!(dets.is_empty(), "{dets:?}");
    }

    #[test]
    fn vendor_prefixes_flagged() {
        let (_, dets) = SecretScanner::redact("use ghp_Abcdef1234567890Abcdef1234567890abcd ok");
        assert!(dets.iter().any(|d| d.kind == SecretKind::ApiToken));
    }

    #[test]
    fn word_boundary_guard_avoids_false_hits() {
        // "compassion=" contains "pass"; must NOT be flagged.
        let (_, dets) = SecretScanner::redact("compassion=forgiveness always wins");
        assert!(dets.is_empty(), "{dets:?}");
        // Short values below the minimum length are ignored.
        let (_, dets) = SecretScanner::redact("password=abc");
        assert!(dets.is_empty());
    }

    #[test]
    fn multibyte_text_survives_scanning() {
        let text = "🔒 пароль password=supersecret99 🔑 done";
        let (clean, dets) = SecretScanner::redact(text);
        assert_eq!(dets.len(), 1);
        assert!(clean.contains('🔒') && clean.contains('🔑'));
        assert!(!clean.contains("supersecret99"));
    }

    #[test]
    fn build_context_respects_permissions_and_caps_length() {
        let perms = ContextPermissions {
            page_content: true,
            open_tabs: false,
            history: true,
            notes: false,
            clipboard: false,
            strip_secrets: true,
            max_chars: 200,
        };
        let pieces = vec![
            ContextPiece {
                source: ContextSource::Page,
                title: "Page".into(),
                body: "public content".into(),
            },
            ContextPiece {
                source: ContextSource::Tabs,
                title: "Tabs".into(),
                body: "should be excluded".into(),
            },
            ContextPiece {
                source: ContextSource::History,
                title: "History".into(),
                body: "visited password=verysecret77 yesterday".into(),
            },
        ];
        let (ctx, stripped) = build_context(&perms, &pieces);
        assert!(ctx.contains("public content"));
        assert!(!ctx.contains("excluded"));
        assert_eq!(stripped, 1);
        assert!(ctx.chars().count() <= 200);
    }

    #[test]
    fn actions_parse_from_fenced_blocks() {
        let reply = "Sure.\n```apb-action\norganize-tabs\ncreate-note \"Meeting notes\"\nopen-url https://example.com/doc\nbogus-line\n```\nDone.";
        let acts = parse_actions(reply);
        assert_eq!(
            acts,
            vec![
                AiAction::OrganizeTabs,
                AiAction::CreateNote { title: "Meeting notes".into() },
                AiAction::OpenUrl { url: "https://example.com/doc".into() },
            ]
        );
        assert!(!acts[0].requires_confirmation());
        assert!(acts[1].requires_confirmation() && acts[2].requires_confirmation());
    }

    #[derive(Clone)]
    struct MockTransport {
        last_body: std::sync::Arc<Mutex<Option<String>>>,
    }

    impl MockTransport {
        fn new() -> Self {
            Self { last_body: std::sync::Arc::new(Mutex::new(None)) }
        }
    }

    impl HttpTransport for MockTransport {
        fn post_json(&self, _url: &str, _h: &[(String, String)], body: &str) -> Result<String> {
            *self.last_body.lock().unwrap() = Some(body.to_string());
            Ok(
                r##"{"choices":[{"message":{"role":"assistant","content":"Here is my plan.\n```apb-action\nsummarize-page\n```\nAnd your AWS key was removed."}}]}"##
                    .to_string(),
            )
        }
    }

    #[test]
    fn client_pipeline_scrubs_outbound_and_parses_reply() {
        let cfg = ProviderConfig::default(); // Ollama => local
        let mock = MockTransport::new();
        let client = AiClient::new(cfg, mock.clone());
        let msgs = vec![ChatMessage::user("my key is AKIAIOSFODNN7EXAMPLE summarize this")];
        let report = client.chat(&msgs, "page text").unwrap();

        assert_eq!(report.secrets_blocked, 1);
        assert!(report.provider_local);
        assert_eq!(report.actions, vec![AiAction::SummarizePage]);

        let sent = mock.last_body.lock().unwrap().clone().unwrap();
        assert!(!sent.contains("AKIAIOSFODNN7EXAMPLE"), "secret leaked upstream!");
        assert!(sent.contains("[REDACTED:aws-access-key]"));
        assert!(sent.contains("\"model\":\"llama3.2\""));
    }

    #[test]
    fn anthropic_payload_uses_system_field_and_x_api_key_header() {
        #[derive(Clone)]
        struct Capture(std::sync::Arc<Mutex<Option<(String, Vec<(String, String)>)>>>);
        impl HttpTransport for Capture {
            fn post_json(
                &self,
                url: &str,
                h: &[(String, String)],
                body: &str,
            ) -> Result<String> {
                *self.0.lock().unwrap() = Some((format!("{url}|{body}"), h.to_vec()));
                Ok(r#"{"content":[{"type":"text","text":"ok"}]}"#.to_string())
            }
        }
        let capture = Capture(std::sync::Arc::new(Mutex::new(None)));
        let cfg = ProviderConfig {
            kind: ProviderKind::AnthropicCompatible,
            base_url: "https://api.anthropic.com/v1/messages".into(),
            model: "claude-sonnet-4-5".into(),
            api_key_env: Some("APB_TEST_KEY".into()),
            ..Default::default()
        };
        std::env::set_var("APB_TEST_KEY", "test-key-123");
        let client = AiClient::new(cfg, capture.clone());
        let report = client.chat(&[ChatMessage::user("hi")], "").unwrap();
        assert_eq!(report.reply, "ok");
        let (req, headers) = capture.0.lock().unwrap().clone().unwrap();
        assert!(req.contains("\"system\":"));
        assert!(headers.iter().any(|(k, v)| k == "x-api-key" && v == "test-key-123"));
    }

    #[test]
    fn missing_cloud_key_errors_clearly() {
        let cfg = ProviderConfig {
            kind: ProviderKind::OpenAiCompatible,
            base_url: "https://api.openai.com/v1/chat/completions".into(),
            model: "gpt-x".into(),
            api_key_env: Some("DEFINITELY_UNSET_VAR_XYZ".into()),
            ..Default::default()
        };
        std::env::remove_var("DEFINITELY_UNSET_VAR_XYZ");
        let client = AiClient::new(cfg, MockTransport::new());
        let err = client.chat(&[ChatMessage::user("hi")], "").unwrap_err();
        assert!(matches!(err, AiError::ApiKeyMissing(_)));
    }

    #[test]
    fn provider_default_is_local_ollama() {
        let cfg = ProviderConfig::default();
        assert_eq!(cfg.kind, ProviderKind::Ollama);
        assert!(cfg.kind.is_local());
        assert!(!ProviderKind::OpenAiCompatible.is_local());
        assert!(cfg.base_url.starts_with("http://127.0.0.1"));
    }
}
