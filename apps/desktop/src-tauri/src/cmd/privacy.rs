// Made by MrDuck && Ox-Alpha
#![allow(unused_imports)]

use crate::cmd::ai::load_ai_config;
use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub(crate) fn get_privacy_overview(state: tauri::State<'_, SharedState>) -> Result<serde_json::Value, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let profile_root = guard.profiles.storage_root(active.profile.id);
    let policy = active.privacy.effective_policy();
    let persona = apb_privacy::FingerprintPersona::derive(
        active.profile.id,
        policy.fingerprint_protection,
    );
    let ai_cfg = load_ai_config(&profile_root);
    let inputs = apb_privacy::AuditInputs {
        dns_encrypted: !matches!(active.network.dns.mode, apb_network::DnsMode::System),
        proxy_configured: active.network.default_chain.is_some(),
        extensions_with_broad_permissions: guard
            .extensions
            .broad_permission_count(active.profile.id),
        ai_cloud_configured: !ai_cfg.kind.is_local(),
        vault_created: active.vault_path.exists(),
    };
    let findings = apb_privacy::run_audit(&policy, &inputs);
    Ok(serde_json::json!({
        "level": active.privacy.policy.level,
        "emergency": active.privacy.emergency_mode,
        "policy": active.privacy.effective_policy(),
        "dashboard": apb_privacy::fingerprint_dashboard(&policy, &persona),
        "findings": findings,
        "stats": active.blocker.stats(),
        "threats": active.privacy.threats,
    }))
}

#[tauri::command]
pub(crate) fn set_privacy_level(
    state: tauri::State<'_, SharedState>,
    level: apb_profiles::PrivacyLevel,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    active.privacy.policy = apb_privacy::PrivacyPolicy::for_level(level);
    // Keep the profile's own level in sync so presets survive restarts.
    active.profile.privacy_level = level;
    guard.persist_active_config()?;
    Ok(())
}

#[tauri::command]
pub(crate) fn update_privacy_policy(
    state: tauri::State<'_, SharedState>,
    policy: apb_privacy::PrivacyPolicy,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    active.privacy.policy = policy;
    guard.persist_active_config()?;
    Ok(())
}

#[tauri::command]
pub(crate) fn set_emergency_mode(
    state: tauri::State<'_, SharedState>,
    on: bool,
) -> Result<bool, String> {
    let mut guard = state.lock().unwrap();
    {
        let active = guard.active_mut_or_err()?;
        active.privacy.emergency_mode = on;
    }
    guard.persist_active_config()?;
    Ok(guard.active_or_err()?.privacy.emergency_mode)
}

#[tauri::command]
pub(crate) fn add_blocklist(
    state: tauri::State<'_, SharedState>,
    name: String,
    category: apb_privacy::TrackerCategory,
    text: String,
) -> Result<usize, String> {
    let mut guard = state.lock().unwrap();
    let added = {
        let active = guard.active_mut_or_err()?;
        let n = active.blocker.add_custom_list(&text, category);
        if n > 0 {
            active.privacy.custom_lists.push(apb_privacy::CustomList { name, category, text });
        }
        n
    };
    guard.persist_active_config()?;
    Ok(added)
}

/// Panic Button (§10A.26): wipe traces of the current session in one shot.
#[tauri::command]
pub(crate) fn panic_button(state: tauri::State<'_, SharedState>) -> Result<Vec<String>, String> {
    let mut guard = state.lock().unwrap();
    let mut done = Vec::new();
    {
        let active = guard.active_mut_or_err()?;
        let actions = active.privacy.panic_actions;

        active.history.clear_all().map_err(|e| e.to_string())?;
        done.push("история текущей сессии очищена".into());

        active.blocker.reset_stats();
        done.push("счётчики блокировок сброшены".into());

        if actions.clear_all_temporary_data || actions.close_anonymous_session {
            if active.vault.is_some() {
                if let Some(v) = active.vault.as_mut() {
                    v.lock();
                }
                active.vault = None;
                done.push("сейф заблокирован".into());
            }
        }

        active.privacy.emergency_mode = true;
        done.push("Emergency Privacy Mode включён".into());
    }
    guard.persist_active_config()?;
    Ok(done)
}

// ---------------------------------------------------------------------
// Network (§10A.6 — DoH/DoT, proxy chains, route preview)
// ---------------------------------------------------------------------

// Made by MrDuck && Ox-Alpha