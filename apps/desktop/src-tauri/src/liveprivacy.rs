// Made by MrDuck && Ox-Alpha
//! Live privacy filter shared between command handlers, the WebView2 tab
//! factory and the local filtering proxy.
//!
//! Why a separate structure instead of `AppState`: request handling runs on
//! hot paths (every network request of every tab through the proxy). Locking
//! the big `SharedState` mutex there risks UI-thread stalls, so the filter
//! keeps an atomically swappable snapshot (`policy` + its own
//! `TrackerBlocker` with built-in rules, user lists and block statistics).
//! Snapshots are rebuilt whenever the profile switches or privacy settings
//! change — cheap, lock-free reads for the data plane.

use apb_privacy::{CustomList, PrivacyLevel, PrivacyPolicy, TrackerBlocker};
use std::sync::{Arc, OnceLock, RwLock};

/// Immutable settings snapshot used by the data plane.
pub struct PrivacySnap {
    pub policy: PrivacyPolicy,
    pub blocker: TrackerBlocker,
}

impl PrivacySnap {
    /// Allow-everything snapshot used until the profile system boots.
    pub fn allow_all() -> Self {
        let mut policy = PrivacyPolicy::for_level(PrivacyLevel::Standard);
        policy.block_trackers = false;
        policy.block_ads = false;
        policy.block_malicious_domains = false;
        Self { policy, blocker: TrackerBlocker::new() }
    }

    /// Full snapshot: effective policy (+ emergency merge done by caller)
    /// plus a blocker loaded with the profile's custom lists.
    pub fn build(policy: PrivacyPolicy, lists: &[CustomList]) -> Self {
        let mut blocker = TrackerBlocker::new();
        for l in lists {
            blocker.add_custom_list(&l.text, l.category);
        }
        Self { policy, blocker }
    }
}

/// Atomically swappable snapshot holder. Reads are wait-free clones of an Arc.
#[derive(Default)]
pub struct LiveFilter(RwLock<Option<Arc<PrivacySnap>>>);

impl LiveFilter {
    pub fn get(&self) -> Arc<PrivacySnap> {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or_else(|| Arc::new(PrivacySnap::allow_all()))
    }

    pub fn set(&self, snap: PrivacySnap) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(snap));
    }
}

// ---------------------------------------------------------------------------
// Unified WebView2 browser arguments
// ---------------------------------------------------------------------------
//
// All webviews of one user-data folder MUST be created with IDENTICAL
// environment options (WebView2 refuses otherwise), so the shell window and
// every tab share this exact string, computed once at startup.

const DEFAULT_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

static BROWSER_ARGS: OnceLock<String> = OnceLock::new();

/// Must be called once before the first webview is created.
pub fn init_browser_args(proxy_port: Option<u16>) {
    let mut args = DEFAULT_ARGS.to_string();
    if let Some(port) = proxy_port {
        args.push_str(&format!(" --proxy-server=http://127.0.0.1:{port}"));
    }
    let _ = BROWSER_ARGS.set(args);
}

/// The session-wide browser arguments (safe to call from any thread).
pub fn browser_args() -> &'static str {
    BROWSER_ARGS
        .get()
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_ARGS)
}

// ---------------------------------------------------------------------------
// Snapshot refresh
// ---------------------------------------------------------------------------

/// Rebuild the live snapshot from the active profile's privacy settings.
/// Call after every mutation that affects blocking: level/policy updates,
/// emergency mode toggles, custom list changes, profile switch/delete.
pub fn sync_from_state(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let state = app.state::<crate::state::SharedState>();
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let policy = active.privacy.effective_policy();
    let lists = active.privacy.custom_lists.clone();
    drop(guard);

    let filter = app.state::<Arc<LiveFilter>>();
    filter.set(PrivacySnap::build(policy, &lists));
    Ok(())
}

// Made by MrDuck && Ox-Alpha
