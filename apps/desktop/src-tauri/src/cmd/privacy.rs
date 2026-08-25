// Made by MrDuck && Ox-Alpha
#![allow(unused_imports)]

use crate::cmd::ai::load_ai_config;
use crate::liveprivacy::{LiveFilter, PrivacySnap};
use crate::state::{AppState, SharedState};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub(crate) fn get_privacy_overview(
    state: tauri::State<'_, SharedState>,
    filter: tauri::State<'_, Arc<LiveFilter>>,
) -> Result<serde_json::Value, String> {
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
    let snap: Arc<PrivacySnap> = filter.get();
    let live = snap.blocker.stats();
    Ok(serde_json::json!({
        "level": active.privacy.policy.level,
        "emergency": active.privacy.emergency_mode,
        "policy": active.privacy.effective_policy(),
        "dashboard": apb_privacy::fingerprint_dashboard(&policy, &persona),
        "findings": findings,
        "stats": {
            "total_blocked": live.total_blocked,
            "per_category": live.per_category,
            "per_site": top_sites(&live.per_site, 10),
            "rules": snap.blocker.rule_count(),
            "proxy_live": true,
        },
        "custom_lists": active.privacy.custom_lists,
        "threats": active.privacy.threats,
    }))
}

fn top_sites(per_site: &std::collections::BTreeMap<String, u64>, n: usize) -> Vec<serde_json::Value> {
    let mut v: Vec<(String, u64)> = per_site.iter().map(|(k, c)| (k.clone(), *c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(n);
    v.into_iter()
        .map(|(site, count)| serde_json::json!({ "site": site, "count": count }))
        .collect()
}

/// Live counters of the filtering proxy (network-layer blocks).
#[tauri::command]
pub(crate) fn privacy_stats(filter: tauri::State<'_, Arc<LiveFilter>>) -> Result<serde_json::Value, String> {
    let snap: Arc<PrivacySnap> = filter.get();
    let st = snap.blocker.stats();
    Ok(serde_json::json!({
        "total_blocked": st.total_blocked,
        "per_category": st.per_category,
        "per_site": top_sites(&st.per_site, 10),
        "rules": snap.blocker.rule_count(),
    }))
}

#[tauri::command]
pub(crate) fn privacy_reset_stats(filter: tauri::State<'_, Arc<LiveFilter>>) -> Result<(), String> {
    filter.get().blocker.reset_stats();
    Ok(())
}

#[tauri::command]
pub(crate) fn set_privacy_level(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    level: apb_profiles::PrivacyLevel,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    active.privacy.policy = apb_privacy::PrivacyPolicy::for_level(level);
    // Keep the profile's own level in sync so presets survive restarts.
    active.profile.privacy_level = level;
    guard.persist_active_config()?;
    drop(guard);
    crate::liveprivacy::sync_from_state(&app)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn update_privacy_policy(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    policy: apb_privacy::PrivacyPolicy,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    active.privacy.policy = policy;
    guard.persist_active_config()?;
    drop(guard);
    crate::liveprivacy::sync_from_state(&app)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn set_emergency_mode(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    on: bool,
) -> Result<bool, String> {
    let mut guard = state.lock().unwrap();
    {
        let active = guard.active_mut_or_err()?;
        active.privacy.emergency_mode = on;
    }
    guard.persist_active_config()?;
    let now = guard.active_or_err()?.privacy.emergency_mode;
    drop(guard);
    crate::liveprivacy::sync_from_state(&app)?;
    Ok(now)
}

#[tauri::command]
pub(crate) fn add_blocklist(
    app: AppHandle,
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
    drop(guard);
    crate::liveprivacy::sync_from_state(&app)?;
    Ok(added)
}

/// Удалить пользовательский список по имени и пересобрать базу доменов
/// из оставшихся (builtin-правила возвращаются автоматически через
/// TrackerBlocker::new). Живой фильтр синхронизируется сразу.
#[tauri::command]
pub(crate) fn remove_blocklist(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    name: String,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    {
        let active = guard.active_mut_or_err()?;
        let before = active.privacy.custom_lists.len();
        active.privacy.custom_lists.retain(|l| l.name != name);
        if active.privacy.custom_lists.len() == before {
            return Err(format!("список «{name}» не найден"));
        }
        let mut blocker = apb_privacy::TrackerBlocker::new();
        for l in &active.privacy.custom_lists {
            blocker.add_custom_list(&l.text, l.category);
        }
        active.blocker = blocker;
    }
    guard.persist_active_config()?;
    drop(guard);
    crate::liveprivacy::sync_from_state(&app)?;
    Ok(())
}

/// Panic Button (§10A.26): wipe traces of the current session in one shot.
#[tauri::command]
pub(crate) fn panic_button(
    app: AppHandle,
    state: tauri::State<'_, SharedState>,
    filter: tauri::State<'_, Arc<LiveFilter>>,
) -> Result<Vec<String>, String> {
    let mut guard = state.lock().unwrap();
    let mut done = Vec::new();
    {
        let active = guard.active_mut_or_err()?;
        let actions = active.privacy.panic_actions;

        active.history.clear_all().map_err(|e| e.to_string())?;
        done.push("история текущей сессии очищена".into());

        active.blocker.reset_stats();
        filter.get().blocker.reset_stats();
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
    drop(guard);
    crate::liveprivacy::sync_from_state(&app)?;
    Ok(done)
}

// ---------------------------------------------------------------------
// Network (§10A.6 — DoH/DoT, proxy chains, route preview)
// ---------------------------------------------------------------------

// Made by MrDuck && Ox-Alpha