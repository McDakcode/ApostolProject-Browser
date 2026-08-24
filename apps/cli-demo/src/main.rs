//! apb-cli-demo — сквозной интеграционный прогон backend-слоя APB.
//!
//! Создаёт временный data root и проходит по всем подсистемам: профили,
//! закладки, история, заметки, вкладки/сессии, палитра команд, privacy
//! engine, сеть, сейф, AI firewall (без сети), canvas и расширения.
//! Запуск: `cargo run -p apb-cli-demo`

use apb_ai::{AiAction, ProviderConfig, ProviderKind, SecretScanner};
use apb_bookmarks::BookmarkStore;
use apb_canvas::{CanvasStore, Document};
use apb_commands::{CommandEntry, CommandRegistry};
use apb_extensions::{ExtensionRegistry, Manifest, Permission};
use apb_history::{HistoryStore, RecordingPolicy};
use apb_network::{DnsMode, NetworkSettings, ProxyHop, ProxyType};
use apb_notes::Vault as NotesVault;
use apb_privacy::{PrivacyPolicy, PrivacyLevel, PrivacyState, TrackerBlocker};
use apb_profiles::{ProfileManager, StorageMode};
use apb_storage::Store;
use apb_tabs::{Session, TabTree};
use apb_vault::{EntryKind, Vault};
use std::path::PathBuf;

fn main() {
    let root = std::env::temp_dir().join(format!("apb-cli-demo-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    println!("== APB CLI demo, data root: {}", root.display());

    // --- Profiles ---------------------------------------------------------
    let profiles = ProfileManager::open(&root).unwrap();
    let personal = profiles.create("Personal").unwrap();
    let anon = profiles.create_anonymous().unwrap();
    println!("профили: {} + {} (ephemeral)", personal.name, anon.name);
    assert_eq!(anon.storage_mode, StorageMode::Ephemeral);

    // --- Bookmarks / History ----------------------------------------------
    let bm_store = Store::open(root.join("profiles").join(personal.id.to_string()).join("bm.sqlite"), apb_bookmarks::MIGRATIONS).unwrap();
    let bookmarks = BookmarkStore::new(bm_store);
    bookmarks.add("Rust Book", "https://doc.rust-lang.org/book/", None, &["rust", "docs"], None).unwrap();
    assert_eq!(bookmarks.search("rust").unwrap().len(), 1);
    println!("закладки: ок");

    let h_path = root.join("profiles").join(anon.id.to_string()).join("h.sqlite");
    let hist_store = Store::open(&h_path, apb_history::MIGRATIONS).unwrap();
    let history = HistoryStore::new(hist_store, RecordingPolicy::Disabled);
    history.record("https://secret.example", "must not persist").unwrap();
    assert_eq!(history.recent(10).unwrap().len(), 0);
    drop(history);
    println!("история анонимного профиля не пишется на диск: ок");

    // --- Notes --------------------------------------------------------------
    let notes_root = profiles.storage_root(personal.id).join("notes");
    let vault_notes = NotesVault::open(&notes_root, notes_root.join("notes-index.sqlite")).unwrap();
    vault_notes.write_note("Architecture.md", "# Architecture\n\nSee [[Network Layer]] #rust").unwrap();
    assert!(!vault_notes.backlinks("Network Layer").unwrap().is_empty());
    println!("заметки + backlinks: ок");

    // --- Tabs session --------------------------------------------------------
    let mut tree = TabTree::new();
    tree.open_tab("https://example.com", "Example", None);
    let mut session = Session::new();
    session.workspaces.insert(personal.id, tree);
    session.save_to_file(root.join("session.json")).unwrap();
    assert!(Session::load_from_file(root.join("session.json")).is_ok());
    println!("сессия вкладок переживает перезапуск: ок");

    // --- Command palette ------------------------------------------------------
    let mut commands = CommandRegistry::new();
    commands.register(CommandEntry {
        id: "privacy.audit".into(),
        title: "Run Privacy Audit".into(),
        category: "Privacy".into(),
        keywords: vec!["audit".into(), "privacy".into()],
        shortcut: None,
    });
    assert!(!commands.search("audit").is_empty());
    println!("палитра команд: ок");

    // --- Privacy engine ---------------------------------------------------------
    let blocker = TrackerBlocker::new();
    let policy = PrivacyPolicy::for_level(PrivacyLevel::Balanced);
    assert!(blocker.inspect("google-analytics.com", &policy).is_some());
    assert!(blocker.stats().total_blocked >= 1);

    let profile_root = profiles.storage_root(personal.id);
    let mut pstate = PrivacyState::for_profile(&personal);
    pstate.emergency_mode = false;
    pstate.save(&profile_root).unwrap();
    assert!(PrivacyState::load(&profile_root).unwrap().is_some());

    let persona = apb_privacy::FingerprintPersona::derive(personal.id, apb_privacy::FingerprintLevel::Standard);
    assert!(persona.injection_script().contains("hardwareConcurrency"));
    println!("privacy engine (tracker/fingerprint/emergency): ок");

    // --- Network -----------------------------------------------------------------
    let mut net = NetworkSettings::default();
    net.dns.mode = DnsMode::Doh;
    net.dns.doh_url = "https://dns.quad9.net/dns-query".into();
    let chain = net
        .add_chain(
            "work",
            vec![ProxyHop { kind: ProxyType::Socks5, host: "127.0.0.1".into(), port: 1080, username: None, password: None }],
        )
        .unwrap();
    net.default_chain = Some(chain);
    let route = net.effective_route_for("example.com");
    assert!(route.iter().any(|n| n.label == "Socks5"));
    net.save(&profile_root).unwrap();
    println!("network (DoH + цепочка + маршрут): ок");

    // --- Vault ---------------------------------------------------------------------
    let vault_path = root.join("vault.json");
    let mut vault = Vault::create(&vault_path, "demo-passphrase").unwrap();
    vault.add_entry(EntryKind::Password { title: "GH".into(), username: "octo".into(), password: "pw123456".into(), url: None, totp_secret: None }).unwrap();
    assert_eq!(vault.list_summaries().unwrap().len(), 1);
    println!("secure vault (AES-GCM + argon2): ок");

    // --- AI firewall (без реальной сети) --------------------------------------------
    let (clean, dets) = SecretScanner::redact("key AKIAIOSFODNN7EXAMPLE and password=hunter2secret");
    assert_eq!(dets.len(), 2);
    assert!(!clean.contains("AKIAIOSFODNN7EXAMPLE"));
    let cfg = ProviderConfig { kind: ProviderKind::Ollama, ..Default::default() };
    assert!(cfg.kind.is_local());
    let _action = AiAction::OrganizeTabs;
    println!("AI privacy firewall ({} секрета отфильтровано): ок", dets.len());

    // --- Canvas ----------------------------------------------------------------------
    let canvas_store = CanvasStore::open(profiles.storage_root(personal.id).join("canvas")).unwrap();
    let doc = Document::new("Mind map");
    canvas_store.save("Mind map", &doc).unwrap();
    canvas_store.add_link_card("Mind map", "https://example.com/article", "Article").unwrap();
    assert!(canvas_store.load("Mind map").unwrap().to_svg().contains("<svg"));
    println!("canvas (JSON-документ + SVG export): ок");

    // --- Extensions ---------------------------------------------------------------------
    let ext_root = root.join("ext-data");
    std::fs::create_dir_all(ext_root.join("src-ext")).unwrap();
    std::fs::write(
        ext_root.join("src-ext").join("manifest.json"),
        r#"{"id":"demo","name":"Demo","version":"0.1.0","api_version":1,"entry_point":"main.js","permissions":["current_tab"]}"#,
    )
    .unwrap();
    let manifest = Manifest::load_from_dir(ext_root.join("src-ext")).unwrap();
    let mut registry = ExtensionRegistry::open(&ext_root).unwrap();
    registry.install(ext_root.join("src-ext")).unwrap();
    registry.grant_permissions(personal.id, &manifest.id, &[Permission::CurrentTab]).unwrap();
    assert!(registry.can(personal.id, &manifest.id, Permission::CurrentTab));
    assert!(!registry.can(personal.id, &manifest.id, Permission::History));
    println!("extensions (манифест + sandbox): ок");

    // Cleanup
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&ext_root).ok();
    std::fs::remove_file(&vault_path).ok();

    println!("== ВСЁ ОК ==");
}
