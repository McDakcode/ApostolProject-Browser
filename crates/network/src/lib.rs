// Made by MrDuck && Ox-Alpha
//! apb-network
//!
//! Network Settings engine (design doc §9, §10A.6-8): DNS (system / custom /
//! DoH / DoT), proxy chains (multi-hop, SOCKS5/HTTP/HTTPS), per-site routing
//! rules, WebRTC & IPv6 controls, URL-privacy toggles and connection
//! diagnostics.
//!
//! Honest boundary: this crate owns configuration, validation and std-only
//! diagnostics (TCP reachability, system DNS resolution). Actually routing
//! web traffic through a chain is the job of the engine adapter — it reads
//! `effective_route_for(host)` and configures its proxy/DNS stack from it.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, NetworkError>;

// ---------------------------------------------------------------------------
// DNS (§9, §10A.6)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsMode {
    System,
    Custom,
    Doh,
    Dot,
}

/// Well-known DoH endpoints so the user picks by name instead of typing URLs.
pub const KNOWN_DOH_PROVIDERS: &[(&str, &str)] = &[
    ("Cloudflare", "https://cloudflare-dns.com/dns-query"),
    ("Cloudflare (family)", "https://family.cloudflare-dns.com/dns-query"),
    ("Google", "https://dns.google/dns-query"),
    ("Quad9", "https://dns.quad9.net/dns-query"),
    ("Quad9 (secure)", "https://dns9.quad9.net/dns-query"),
    ("AdGuard", "https://dns.adguard-dns.com/dns-query"),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DnsConfig {
    pub mode: DnsMode,
    /// Plain DNS servers for Custom mode (IP strings).
    #[serde(default)]
    pub custom_servers: Vec<String>,
    /// DoH endpoint URL for Doh mode.
    #[serde(default)]
    pub doh_url: String,
    /// DoT server `host:853` for Dot mode.
    #[serde(default)]
    pub dot_server: String,
    /// DNS leak protection: when routing through a proxy, forbid falling
    /// back to system resolution (§10A.6). Advisory today — resolution is
    /// fail-open so a dead resolver never blacks out the web.
    #[serde(default = "default_true")]
    pub prevent_leaks: bool,
    #[serde(default = "default_true")]
    pub cache_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            mode: DnsMode::System,
            custom_servers: Vec::new(),
            doh_url: String::new(),
            dot_server: String::new(),
            prevent_leaks: true,
            cache_enabled: true,
        }
    }
}

impl DnsConfig {
    pub fn is_encrypted(&self) -> bool {
        matches!(self.mode, DnsMode::Doh | DnsMode::Dot)
    }

    pub fn validate(&self) -> Result<()> {
        match self.mode {
            DnsMode::Custom => {
                if self.custom_servers.is_empty() {
                    return Err(NetworkError::Invalid(
                        "Custom DNS выбран, но список серверов пуст".into(),
                    ));
                }
                for s in &self.custom_servers {
                    s.parse::<IpAddr>().map_err(|_| {
                        NetworkError::Invalid(format!("'{s}' не является IP-адресом"))
                    })?;
                }
            }
            DnsMode::Doh => {
                if !self.doh_url.starts_with("https://") {
                    return Err(NetworkError::Invalid(
                        "DoH URL должен начинаться с https://".into(),
                    ));
                }
            }
            DnsMode::Dot => {
                if self.dot_server.is_empty() {
                    return Err(NetworkError::Invalid("DoT сервер не указан".into()));
                }
            }
            DnsMode::System => {}
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Proxy chains (§10A.8)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyType {
    Http,
    Https,
    Socks5,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProxyHop {
    pub kind: ProxyType,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: Option<String>,
    /// Stored only in the profile's own settings file on disk; never synced
    /// or logged. For stronger protection leave empty and use IP auth.
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Made by MrDuck && Ox-Alpha
pub struct ProxyChain {
    pub id: Uuid,
    pub name: String,
    pub hops: Vec<ProxyHop>,
}

impl ProxyChain {
    pub fn validate(&self) -> Result<()> {
        if self.hops.is_empty() {
            return Err(NetworkError::Invalid("Цепочка пуста".into()));
        }
        if self.hops.len() > 3 {
            return Err(NetworkError::Invalid(
                "Максимум 3 узла в цепочке (Browser → P1 → P2 → P3 → Internet)".into(),
            ));
        }
        for hop in &self.hops {
            if hop.host.trim().is_empty() {
                return Err(NetworkError::Invalid("Пустой хост в цепочке".into()));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Routing rules + full settings
// ---------------------------------------------------------------------------

/// Per-host routing: send matching hosts through a specific chain, or force
/// direct connection even when a default chain exists (§10A.6 per-profile /
/// per-workspace routing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingRule {
    /// Host suffix match: "corp.internal" matches "git.corp.internal".
    pub host_suffix: String,
    pub chain_id: Option<Uuid>,
    pub direct: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ipv6Policy {
    Enabled,
    PreferIpv4,
    Disabled,
}

/// URL-privacy mechanisms (§10A.15) — each has an individual switch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlPrivacy {
    pub network_prefetch: bool,
    pub dns_prefetch: bool,
    pub speculative_connections: bool,
    pub link_preloading: bool,
    pub search_suggestions_remote: bool,
    pub crash_reports_ask: bool,
}

impl Default for UrlPrivacy {
    fn default() -> Self {
        // Privacy-first defaults (§10A.15/§10A.17).
        Self {
            network_prefetch: false,
            dns_prefetch: false,
            speculative_connections: false,
            link_preloading: false,
            search_suggestions_remote: false,
            crash_reports_ask: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub dns: DnsConfig,
    pub chains: BTreeMap<Uuid, ProxyChain>,
    pub default_chain: Option<Uuid>,
    pub rules: Vec<RoutingRule>,
    pub ipv6: Ipv6Policy,
    pub url_privacy: UrlPrivacy,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            dns: DnsConfig::default(),
            chains: BTreeMap::new(),
            default_chain: None,
            rules: Vec::new(),
            ipv6: Ipv6Policy::Enabled,
            url_privacy: UrlPrivacy::default(),
        }
    }
}

/// One node of the visualized route (§10A.8 Network Route view).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteNode {
    pub label: String,
    pub detail: String,
}

impl NetworkSettings {
    pub fn save(&self, root: impl AsRef<Path>) -> Result<()> {
        let path: PathBuf = root.as_ref().join("network.json");
        let tmp = root.as_ref().join("network.json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn load(root: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = root.as_ref().join("network.json");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_str(&std::fs::read_to_string(&path)?)?))
    }

    pub fn add_chain(&mut self, name: &str, hops: Vec<ProxyHop>) -> Result<Uuid> {
        let chain = ProxyChain {
            id: Uuid::new_v4(),
            name: name.to_string(),
            hops,
        };
        chain.validate()?;
        let id = chain.id;
        self.chains.insert(id, chain);
        Ok(id)
    }

    /// The route a request to `host` would take right now — used both by the
    /// UI visualization and (in the real app) by the engine adapter.
// Made by MrDuck && Ox-Alpha
    pub fn effective_route_for(&self, host: &str) -> Vec<RouteNode> {
        let mut nodes = vec![RouteNode {
            label: "Browser".into(),
            detail: format!("profile DNS: {:?}", self.dns.mode),
        }];
        let mut chain_id = self.default_chain;
        let mut forced_direct = false;
        for rule in &self.rules {
            if host.ends_with(&rule.host_suffix) || host == rule.host_suffix {
                if rule.direct {
                    forced_direct = true;
                    chain_id = None;
                } else {
                    chain_id = rule.chain_id;
                }
                break;
            }
        }
        if let Some(id) = chain_id.filter(|_| !forced_direct) {
            if let Some(chain) = self.chains.get(&id) {
                for hop in &chain.hops {
                    nodes.push(RouteNode {
                        label: format!("{:?}", hop.kind),
                        detail: format!("{}:{}", hop.host, hop.port),
                    });
                }
            }
        } else {
            nodes.push(RouteNode {
                label: "Direct".into(),
                detail: "без прокси".into(),
            });
        }
        match self.dns.mode {
            DnsMode::Doh => nodes.push(RouteNode {
                label: "DoH".into(),
                detail: self.dns.doh_url.clone(),
            }),
            DnsMode::Dot => nodes.push(RouteNode {
                label: "DoT".into(),
                detail: self.dns.dot_server.clone(),
            }),
            _ => {}
        }
        nodes.push(RouteNode {
            label: "Internet".into(),
            detail: host.to_string(),
        });
        nodes
    }

    /// Whether the current setup claims leak protection (proxy in use +
    /// encrypted DNS or explicit no-fallback flag).
    pub fn leak_protection_active(&self) -> bool {
        (self.default_chain.is_some() && self.dns.prevent_leaks) || self.dns.is_encrypted()
    }
}

// ---------------------------------------------------------------------------
// Diagnostics (std-only, no async runtime needed)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticResult {
    pub check: String,
    pub ok: bool,
    pub detail: String,
    pub duration_ms: u128,
}

/// TCP connect test with timeout — backs "connection diagnostics" (§9).
pub fn tcp_connect(host: &str, port: u16, timeout_ms: u64) -> DiagnosticResult {
    let started = std::time::Instant::now();
    let target = format!("{host}:{port}");
    match target.to_socket_addrs() {
        Ok(mut addrs) => {
            let mut last_err = String::from("no addresses");
            for addr in addrs.by_ref() {
                match TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)) {
                    Ok(_) => {
                        return DiagnosticResult {
                            check: format!("TCP {target}"),
                            ok: true,
                            detail: format!("подключено к {addr}"),
                            duration_ms: started.elapsed().as_millis(),
                        };
                    }
                    Err(e) => last_err = e.to_string(),
                }
            }
            DiagnosticResult {
                check: format!("TCP {target}"),
                ok: false,
                detail: last_err,
                duration_ms: started.elapsed().as_millis(),
            }
        }
        Err(e) => DiagnosticResult {
            check: format!("TCP {target}"),
            ok: false,
            detail: format!("DNS: {e}"),
            duration_ms: started.elapsed().as_millis(),
        },
    }
}

/// System resolver test — shows which IPs the OS resolves a host to. Useful
/// to *see* DNS behavior; note it always uses the system resolver (testing a
/// configured DoH endpoint's reachability is done via `tcp_connect` on its
/// host:443).
pub fn resolve_system(host: &str) -> DiagnosticResult {
    let started = std::time::Instant::now();
    match (host, 0u16).to_socket_addrs() {
        Ok(addrs) => {
            let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
            DiagnosticResult {
                check: format!("DNS {host}"),
                ok: !ips.is_empty(),
                detail: ips.join(", "),
                duration_ms: started.elapsed().as_millis(),
            }
        }
        Err(e) => DiagnosticResult {
            check: format!("DNS {host}"),
            ok: false,
            detail: e.to_string(),
            duration_ms: started.elapsed().as_millis(),
        },
    }
}

/// Full diagnostics suite for the current settings.
// Made by MrDuck && Ox-Alpha
pub fn run_diagnostics(settings: &NetworkSettings) -> Vec<DiagnosticResult> {
    let mut out = Vec::new();
    out.push(resolve_system("example.com"));
    match settings.dns.mode {
        DnsMode::Doh => {
            if let Some(host) = settings
                .dns
                .doh_url
                .strip_prefix("https://")
                .and_then(|s| s.split('/').next())
            {
                out.push(tcp_connect(host, 443, 4000));
            }
        }
        DnsMode::Dot => {
            let host = settings.dns.dot_server.split(':').next().unwrap_or("");
            if !host.is_empty() {
                out.push(tcp_connect(host, 853, 4000));
            }
        }
        DnsMode::Custom => {
            for server in settings.dns.custom_servers.iter().take(3) {
                out.push(tcp_connect(server, 53, 3000));
            }
        }
        DnsMode::System => {}
    }
    if let Some(id) = settings.default_chain {
        if let Some(chain) = settings.chains.get(&id) {
            if let Some(first) = chain.hops.first() {
                out.push(tcp_connect(&first.host, first.port, 5000));
            }
        }
    }
    out.push(tcp_connect("1.1.1.1", 443, 4000));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_validation_rejects_bad_input() {
        let mut dns = DnsConfig::default();
        assert!(dns.validate().is_ok()); // System

        dns.mode = DnsMode::Custom;
        assert!(dns.validate().is_err());
        dns.custom_servers = vec!["1.1.1.1".into(), "not-an-ip".into()];
        assert!(dns.validate().is_err());
        dns.custom_servers = vec!["1.1.1.1".into(), "8.8.8.8".into()];
        assert!(dns.validate().is_ok());

        dns.mode = DnsMode::Doh;
        dns.doh_url = "http://insecure.example".into();
        assert!(dns.validate().is_err());
        dns.doh_url = KNOWN_DOH_PROVIDERS[0].1.into();
        assert!(dns.validate().is_ok());
        assert!(dns.is_encrypted());
    }

    #[test]
    fn chain_validation_rules() {
        let mut s = NetworkSettings::default();
        assert!(s.add_chain("empty", vec![]).is_err());
        assert!(s.add_chain("ok", vec![ProxyHop {
            kind: ProxyType::Socks5,
            host: "127.0.0.1".into(),
            port: 9050,
            username: None,
            password: None,
        }])
        .is_ok());
        let four = vec![
            ProxyHop { kind: ProxyType::Socks5, host: "a".into(), port: 1, username: None, password: None },
            ProxyHop { kind: ProxyType::Http, host: "b".into(), port: 2, username: None, password: None },
            ProxyHop { kind: ProxyType::Https, host: "c".into(), port: 3, username: None, password: None },
            ProxyHop { kind: ProxyType::Http, host: "d".into(), port: 4, username: None, password: None },
        ];
        assert!(s.add_chain("too long", four).is_err());
    }

    #[test]
    fn route_visualization_follows_rules() {
        let mut s = NetworkSettings::default();
        let id = s
            .add_chain("work", vec![ProxyHop {
                kind: ProxyType::Socks5,
                host: "10.0.0.2".into(),
                port: 1080,
                username: None,
                password: None,
            }])
            .unwrap();
        s.default_chain = Some(id);
        s.rules.push(RoutingRule {
            host_suffix: "bank.example".into(),
            chain_id: None,
            direct: true,
        });

        let normal = s.effective_route_for("news.example");
        assert_eq!(normal[0].label, "Browser");
        assert_eq!(normal[1].label, "Socks5");
        assert_eq!(normal.last().unwrap().label, "Internet");

        let bank = s.effective_route_for("www.bank.example");
        assert!(bank.iter().any(|n| n.label == "Direct"));
        assert!(!bank.iter().any(|n| n.label == "Socks5"));

        assert!(s.leak_protection_active()); // chain + prevent_leaks default
    }

    #[test]
    fn settings_roundtrip_via_file() {
        let tmp = std::env::temp_dir().join(format!("apb-net-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut s = NetworkSettings::default();
        s.dns.mode = DnsMode::Doh;
        s.dns.doh_url = "https://dns.quad9.net/dns-query".into();
        s.save(&tmp).unwrap();
        let loaded = NetworkSettings::load(&tmp).unwrap().unwrap();
        assert_eq!(loaded.dns.mode, DnsMode::Doh);
        assert!(loaded.dns.is_encrypted());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn diagnostics_return_structured_results() {
        let s = NetworkSettings::default();
        let results = run_diagnostics(&s);
        assert!(!results.is_empty());
        // Every result must carry a human-readable detail either way.
        for r in &results {
            assert!(!r.detail.is_empty());
        }
    }
}

// Made by MrDuck && Ox-Alpha