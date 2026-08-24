//! Shared application state for the Tauri shell.
//!
//! Holds the `ProfileManager` (device-wide) plus the currently *active*
//! profile's opened stores (bookmarks/history/notes/privacy/network/canvas/
//! vault). Switching profiles re-opens these against the new profile's
//! isolated storage root вЂ” nothing from one profile's stores leaks into
//! another's, matching design doc В§5 and В§10A.27 (privacy settings are
//! profile-scoped data).
//!
//! The extension registry is device-scoped installs + per-profile grants,
//! so it lives on `AppState`, not in `ActiveProfile`.

use apb_bookmarks::BookmarkStore;
use apb_commands::CommandRegistry;
use apb_history::{HistoryStore, RecordingPolicy};
use apb_network::NetworkSettings;
use apb_notes::Vault as NotesVault;
use apb_extensions::ExtensionRegistry;
use apb_privacy::{PrivacyState, TrackerBlocker};
use apb_profiles::{Profile, ProfileManager, StorageMode};
use apb_storage::Store;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct ActiveProfile {
    pub profile: Profile,
    pub bookmarks: BookmarkStore,
    pub history: HistoryStore,
    pub notes: NotesVault,
    /// Privacy Engine state (В§10): policy + overrides + panic config.
    pub privacy: PrivacyState,
    /// Domain blocker fed with built-in rules + user's custom lists.
    pub blocker: TrackerBlocker,
    /// DNS / proxy / routing configuration (В§10A network section).
    pub network: NetworkSettings,
    /// Secure Vault handle — `None` until created/unlocked this session.
    pub vault: Option<apb_vault::Vault>,
    pub vault_path: PathBuf,
}

pub struct AppState {
    pub profiles: ProfileManager,
    pub active: Option<ActiveProfile>,
    pub commands: CommandRegistry,
    pub extensions: ExtensionRegistry,
}

pub type SharedState = Mutex<AppState>;

impl AppState {
    pub fn bootstrap(data_root: PathBuf) -> Result<Self, String> {
        let profiles = ProfileManager::open(&data_root).map_err(|e| e.to_string())?;
        let mut list = profiles.list().map_err(|e| e.to_string())?;

        if list.is_empty() {
            profiles.create("Personal").map_err(|e| e.to_string())?;
            list = profiles.list().map_err(|e| e.to_string())?;
        }

        let extensions = ExtensionRegistry::open(&data_root).map_err(|e| e.to_string())?;

        let mut state = Self {
            profiles,
            active: None,
            commands: build_command_registry(),
            extensions,
        };
        let first = list.into_iter().next().unwrap();
        state.activate(first.id)?;
        Ok(state)
    }

    pub fn activate(&mut self, profile_id: uuid::Uuid) -> Result<(), String> {
        let profile = self
            .profiles
            .list()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "profile not found".to_string())?;

        let root = self.profiles.storage_root(profile.id);
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;

        let bm_store = Store::open(root.join("bookmarks.sqlite"), apb_bookmarks::MIGRATIONS)
            .map_err(|e| e.to_string())?;
        let bookmarks = BookmarkStore::new(bm_store);

        let hist_store = Store::open(root.join("history.sqlite"), apb_history::MIGRATIONS)
            .map_err(|e| e.to_string())?;
        let policy = match profile.storage_mode {
            StorageMode::Ephemeral => RecordingPolicy::Disabled,
            StorageMode::Persistent => RecordingPolicy::Enabled,
        };
        let history = HistoryStore::new(hist_store, policy);

        let notes = NotesVault::open(root.join("notes"), root.join("notes-index.sqlite"))
            .map_err(|e| e.to_string())?;

        // Privacy Engine: saved state or preset derived from the profile.
        let privacy = match PrivacyState::load(&root) {
            Ok(Some(st)) => st,
            _ => PrivacyState::for_profile(&profile),
        };
        // Blocker: built-in rules + every persisted custom list applied.
        let mut blocker = TrackerBlocker::new();
        for list in &privacy.custom_lists {
            blocker.add_custom_list(&list.text, list.category);
        }

        let network = NetworkSettings::load(&root)
            .ok()
            .flatten()
            .unwrap_or_default();

        self.active = Some(ActiveProfile {
            profile,
            bookmarks,
            history,
            notes,
            privacy,
            blocker,
            network,
            vault: None,
            vault_path: root.join("vault.json"),
        });
        Ok(())
    }

    pub fn active_or_err(&self) -> Result<&ActiveProfile, String> {
        self.active.as_ref().ok_or_else(|| "no active profile".to_string())
    }

    pub fn active_mut_or_err(&mut self) -> Result<&mut ActiveProfile, String> {
        self.active.as_mut().ok_or_else(|| "no active profile".to_string())
    }

    /// Persist privacy + network state of the active profile to its root.
    pub fn persist_active_config(&self) -> Result<(), String> {
        let active = self.active_or_err()?;
        let root = self.profiles.storage_root(active.profile.id);
        active.privacy.save(&root).map_err(|e| e.to_string())?;
        active.network.save(&root).map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn build_command_registry() -> CommandRegistry {
    use apb_commands::CommandEntry;
    let mut r = CommandRegistry::new();
    let mut add = |id: &str, title: &str, category: &str, keywords: &[&str], shortcut: Option<&str>| {
        r.register(CommandEntry {
            id: id.into(),
            title: title.into(),
            category: category.into(),
            keywords: keywords.iter().map(|s| s.to_string()).collect(),
            shortcut: shortcut.map(str::to_string),
        });
    };
    add("tabs.new", "Новая вкладка", "Вкладки", &["new", "tab", "open", "новая", "вкладка", "открыть"], Some("Ctrl+T"));
    add("notes.create", "Создать заметку", "Заметки", &["note", "markdown", "new", "заметка", "маркетдайн", "новая"], Some("Ctrl+Shift+O"));
    add("bookmarks.add", "Добавить закладку", "Закладки", &["bookmark", "save", "закладка", "сохранить"], Some("Ctrl+D"));
    add("profiles.switch", "Переключить профиль", "Профили", &["profile", "switch", "профиль", "переключить"], None);
    add("history.clear", "Очистить данные браузера", "Приватность", &["clear", "cookies", "history", "очистить", "история", "куки"], None);
    add("privacy.emergency", "Режим экстренной приватности", "Приватность", &["emergency", "panic", "экстренный", "паника"], Some("Ctrl+Shift+E"));
    add("privacy.audit", "Запустить аудит приватности", "Приватность", &["audit", "check", "аудит", "проверка"], None);
    add("vault.lock", "Заблокировать сейф", "Сейф", &["vault", "lock", "сейф", "пароли", "блокировка"], None);
    add("ai.chat", "Открыть AI-ассистент", "AI", &["ai", "assistant", "chat", "ии", "чат", "ассистент"], None);
    add("panic.button", "Паника — стереть следы сессии", "Приватность", &["panic", "wipe", "паника", "стереть"], None);
    r
}
