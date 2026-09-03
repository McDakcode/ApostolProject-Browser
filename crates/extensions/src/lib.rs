// Made by MrDuck && Ox-Alpha
//! apb-extensions
//!
//! Own extension system (design doc §12, §10A.22-23): declarative manifest,
//! explicit permission grants per profile, and a sandbox policy that denies
//! everything not granted. Web content, filesystem, vault and AI context are
//! unreachable unless the corresponding permission was granted *and* the
//! dangerous ones were explicitly approved by the user.
//!
//! Package format: a directory (installable as `.extension` folder or unzipped
//! archive) containing `manifest.json` plus assets. Marketplace integration
//! (§29) is a future transport — the registry below already works fully
//! offline with local installs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("extension not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, ExtensionError>;

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// The API surface version this runtime implements. Extensions targeting an
/// older compatible major are accepted; newer ones are refused loudly.
pub const SUPPORTED_API_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    CurrentTab,
    SelectedText,
    AllWebsites,
    Cookies,
    History,
    Bookmarks,
    Downloads,
    Notes,
    Canvas,
    Network,
    Filesystem,
    ClipboardWrite,
    Notifications,
    AiContext,
}

impl Permission {
    pub fn label(&self) -> &'static str {
        match self {
            Permission::CurrentTab => "Содержимое текущей вкладки",
            Permission::SelectedText => "Выделенный текст",
            Permission::AllWebsites => "Данные со всех сайтов",
            Permission::Cookies => "Cookies",
            Permission::History => "История",
            Permission::Bookmarks => "Закладки",
            Permission::Downloads => "Загрузки",
            Permission::Notes => "Заметки",
            Permission::Canvas => "Канвасы",
            Permission::Network => "Сетевые запросы",
            Permission::Filesystem => "Файловая система",
            Permission::ClipboardWrite => "Запись в буфер обмена",
            Permission::Notifications => "Уведомления",
            Permission::AiContext => "AI-контекст",
        }
    }

    /// Dangerous permissions require a separate explicit confirmation dialog
    /// (§10A.22: "This extension can read data from every website...").
    pub fn is_dangerous(&self) -> bool {
        matches!(
            self,
            Permission::AllWebsites
                | Permission::Cookies
                | Permission::History
                | Permission::Filesystem
                | Permission::Network
                | Permission::AiContext
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Relative path of the entry script/asset inside the package.
    pub entry_point: String,
    /// URL match patterns for the content-script runtime (§12 "content
    /// scripts by masks"). Empty = the entry point never runs. Examples:
    /// `https://*/*`, `https://*.example.com/*`, `*://example.com/*`.
    #[serde(default)]
    pub matches: Vec<String>,
    /// Run the content script in every iframe as well (all_frames=false
    /// by default, top frame only — same spirit as WebExtension manifests).
    #[serde(default)]
    pub all_frames: bool,
}

impl Manifest {
    pub fn parse(json: &str) -> Result<Self> {
        let m: Self = serde_json::from_str(json)?;
        m.validate()?;
        Ok(m)
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() || !self.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(ExtensionError::Invalid(
                "manifest.id должен быть непустым и содержать [a-zA-Z0-9-_]".into(),
            ));
        }
        if self.name.trim().is_empty() {
            return Err(ExtensionError::Invalid("manifest.name пуст".into()));
        }
        if self.version.trim().is_empty() {
            return Err(ExtensionError::Invalid("manifest.version пуст".into()));
        }
        if self.api_version > SUPPORTED_API_VERSION {
            return Err(ExtensionError::Invalid(format!(
                "расширение требует API v{}, поддерживается v{SUPPORTED_API_VERSION}",
                self.api_version
            )));
        }
        if self.entry_point.contains("..") || self.entry_point.starts_with('/') {
            return Err(ExtensionError::Invalid(
                "manifest.entry_point не может выходить за пределы пакета".into(),
            ));
        }
        for m in &self.matches {
            match_pattern::compile(m)
                .map_err(|e| ExtensionError::Invalid(format!("manifest.matches: {e}")))?;
        }
        Ok(())
    }

    /// True when this extension's content script applies to `url`
    /// (any of its match patterns matches; an empty `matches` list never
    /// matches anything — the script only runs on granted sites).
    pub fn matches_url(&self, url: &str) -> bool {
        self.matches.iter().any(|p| match_pattern::matches(p, url))
    }

    /// Read the entry-point script from the installed package directory.
    pub fn read_entry_script(&self, install_dir: &Path) -> Result<String> {
        if self.entry_point.is_empty() {
            return Err(ExtensionError::Invalid("entry_point пуст".into()));
        }
        if self.entry_point.split(['/', '\\']).any(|seg| seg == "..") {
            return Err(ExtensionError::Invalid(
                "entry_point не может выходить за пределы пакета".into(),
            ));
        }
        let path = install_dir.join(&self.entry_point);
        if !path.is_file() {
            return Err(ExtensionError::Invalid(format!(
                "файл entry_point не найден: {}",
                path.display()
            )));
        }
        std::fs::read_to_string(&path)
            .map_err(|e| ExtensionError::Invalid(format!("{}: {e}", path.display())))
    }

// Made by MrDuck && Ox-Alpha
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let path = dir.as_ref().join("manifest.json");
        let json = std::fs::read_to_string(&path)
            .map_err(|e| ExtensionError::Invalid(format!("{}: {e}", path.display())))?;
        Self::parse(&json)
    }
}

// ---------------------------------------------------------------------------
// Match patterns (WebExtension-подобный упрощённый формат)
// ---------------------------------------------------------------------------

/// `<scheme>://<host><path>`: scheme = `http|https|*` (звёздочка = оба),
/// host = `*` (любой) | `*.example.com` (домен + поддомены) | точный хост,
/// path = `/...` с глобами `*` (например `/*`, `/foo/*bar`).
/// Используется манифестом (`matches`) и инжект-рантамом в cmd/pages.rs.
pub(crate) mod match_pattern {
    use super::ExtensionError;

    struct Pat {
        scheme_wild: bool,
        scheme: String,
        host_wild: bool,
        host_suffix: Option<String>,
        host_exact: String,
        path: String,
    }

    fn parse(pattern: &str) -> Option<Pat> {
        let p = pattern.trim();
        if p.contains(char::is_whitespace) || p.contains("..") {
            return None;
        }
        let (scheme, rest) = p.split_once("://")?;
        if scheme != "http" && scheme != "https" && scheme != "*" {
            return None;
        }
        let (host, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => return None, // path обязателен, хотя бы «/»
        };
        if host.is_empty() || path.is_empty() {
            return None;
        }
        let (host_wild, host_suffix, host_exact) = if host == "*" {
            (true, None, String::new())
        } else if let Some(sfx) = host.strip_prefix("*.") {
            if sfx.is_empty() || sfx.contains('*') {
                return None;
            }
            (false, Some(sfx.to_string()), sfx.to_string())
        } else if host.contains('*') {
            // звёздочка в хосте разрешена только как префикс «*.»
            return None;
        } else {
            (false, None, host.to_string())
        };
        Some(Pat {
            scheme_wild: scheme == "*",
            scheme: scheme.to_string(),
            host_wild,
            host_suffix,
            host_exact,
            path: path.to_string(),
        })
    }

    /// Синтаксическая проверка (вызывается из Manifest::validate).
    pub fn compile(pattern: &str) -> Result<(), ExtensionError> {
        if parse(pattern).is_none() {
            return Err(ExtensionError::Invalid(format!(
                "«{pattern}» — ожидается <scheme>://<host>/<path>, scheme = http|https|*, host = *|*.example.com|example.com"
            )));
        }
        Ok(())
    }

    /// Проверить URL против паттерна. URL перед разбором НЕ нормализуется —
    /// зови с тем, что даёт движок (location.href и т.п.).
    pub fn matches(pattern: &str, url: &str) -> bool {
        let Some(p) = parse(pattern) else { return false };
        let Some((scheme, rest)) = url.split_once("://") else { return false };
        if p.scheme_wild {
            if scheme != "http" && scheme != "https" {
                return false;
            }
        } else if p.scheme != scheme {
            return false;
        }
        let (host, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        if !p.host_wild {
            if let Some(sfx) = &p.host_suffix {
                if host != sfx && !host.ends_with(&format!(".{sfx}")) {
                    return false;
                }
            } else if host != p.host_exact {
                return false;
            }
        }
        glob_match(&p.path, path)
    }

    /// Простой глоб по `*`: части между звёздочками идут по порядку, первая —
    /// префикс, последняя — суффикс (семантика WebExtension path match).
    fn glob_match(glob: &str, mut s: &str) -> bool {
        let parts: Vec<&str> = glob.split('*').collect();
        if parts.len() == 1 {
            return glob == s;
        }
        let first = parts[0];
        if !s.starts_with(first) {
            return false;
        }
        s = &s[first.len()..];
        let last = parts[parts.len() - 1];
        if s.len() < last.len() {
            return false;
        }
        let mid = &s[..s.len() - last.len()];
        let mut search = mid;
        for part in &parts[1..parts.len() - 1] {
            if part.is_empty() {
                continue;
            }
            match search.find(part) {
                Some(i) => search = &search[i + part.len()..],
                None => return false,
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Registry + grants
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub manifest: Manifest,
    pub install_dir: PathBuf,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub enabled_globally: bool,
}

/// Per-profile decision state (§12 per-profile extensions).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grant {
    pub enabled: bool,
    pub approved_permissions: Vec<Permission>,
    /// User saw and accepted the danger confirmation for these:
    pub dangerous_approved: Vec<Permission>,
}

impl Grant {
    fn new() -> Self {
        Self { enabled: false, approved_permissions: Vec::new(), dangerous_approved: Vec::new() }
    }
}

/// Sandbox capability report — what the runtime will actually give the
/// extension (§10A.23). Everything not listed is denied by construction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SandboxPolicy {
    pub extension_id: String,
    pub capabilities: Vec<String>,
    pub denied_by_default: Vec<&'static str>,
}

/// One injectable content script produced by `ExtensionRegistry::
/// content_scripts_for` — the runtime payload consumed by cmd/pages.rs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentScript {
    pub extension_id: String,
    pub extension_name: String,
    /// Run in every frame (top frame only when false — the default).
    pub all_frames: bool,
    pub code: String,
    /// Permissions approved for the active profile — the runtime exposes
    /// only the matching part of the in-page `apb` API surface.
    pub granted: Vec<Permission>,
}

pub struct ExtensionRegistry {
    root: PathBuf,
    installed: BTreeMap<String, InstalledExtension>,
    /// (profile_id, extension_id) -> grant
    grants: BTreeMap<(Uuid, String), Grant>,
}

impl ExtensionRegistry {
    pub fn open(data_root: impl AsRef<Path>) -> Result<Self> {
        let root = data_root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("extensions"))?;
        let state_path = root.join("extensions").join("registry.json");
        if state_path.exists() {
            let raw = std::fs::read_to_string(&state_path)?;
            let parsed: StoredState = serde_json::from_str(&raw)?;
            Ok(Self {
                root,
                installed: parsed.installed,
                grants: parsed.grants.into_iter().collect(),
            })
        } else {
            Ok(Self { root, installed: BTreeMap::new(), grants: BTreeMap::new() })
        }
    }

    pub fn save(&self) -> Result<()> {
        let state = StoredState {
            installed: self.installed.clone(),
            grants: self.grants.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        };
        let path = self.root.join("extensions").join("registry.json");
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&state)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Install from a source directory containing manifest.json. Files are
    /// copied into `<data>/extensions/installed/<id>-<version>/`.
    pub fn install(&mut self, source_dir: impl AsRef<Path>) -> Result<InstalledExtension> {
        let manifest = Manifest::load_from_dir(source_dir.as_ref())?;
        if let Some(existing) = self.installed.get(&manifest.id) {
            if existing.manifest.version == manifest.version {
                return Err(ExtensionError::Invalid(format!(
                    "расширение {} {} уже установлено",
                    manifest.id, manifest.version
                )));
            }
        }
        let dest = self
            .root
            .join("extensions")
            .join("installed")
            .join(format!("{}-{}", manifest.id, manifest.version));
        copy_dir_recursive(source_dir.as_ref(), &dest)?;
        let record = InstalledExtension {
            manifest,
            install_dir: dest,
            installed_at: chrono::Utc::now(),
            enabled_globally: true,
        };
        // Fresh install starts disabled for every profile until granted (§10A.22).
        self.installed.insert(record.manifest.id.clone(), record.clone());
        self.save()?;
        Ok(record)
    }

    pub fn uninstall(&mut self, id: &str) -> Result<()> {
        let rec = self.installed.remove(id).ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;
        if rec.install_dir.exists() {
            std::fs::remove_dir_all(&rec.install_dir)?;
        }
        self.grants.retain(|(_, ext_id), _| ext_id != id);
        self.save()?;
        Ok(())
    }

    pub fn list(&self) -> Vec<&InstalledExtension> {
        self.installed.values().collect()
    }

// Made by MrDuck && Ox-Alpha
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let rec = self.installed.get_mut(id).ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;
        rec.enabled_globally = enabled;
        self.save()?;
        Ok(())
    }

    /// Approve permissions for a profile. Dangerous ones must come through
    /// `approve_dangerous` after the UI confirmation dialog.
    pub fn grant_permissions(&mut self, profile: Uuid, id: &str, perms: &[Permission]) -> Result<Grant> {
        let rec = self.installed.get(id).ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;
        let declared: Vec<Permission> = rec.manifest.permissions.clone();
        for p in perms {
            if !declared.contains(p) {
                return Err(ExtensionError::Invalid(format!(
                    "разрешение {:?} не заявлено в манифесте расширения {id}",
                    p
                )));
            }
        }
        let grant = self.grants.entry((profile, id.to_string())).or_insert_with(Grant::new);
        grant.enabled = true;
        grant.approved_permissions = perms.to_vec();
        grant.dangerous_approved.retain(|d| perms.contains(d));
        let clone = grant.clone();
        self.save()?;
        Ok(clone)
    }

    /// Enable/disable an extension for one profile without touching grants.
    pub fn set_profile_enabled(&mut self, profile: Uuid, id: &str, enabled: bool) -> Result<()> {
        let grant = self.grants.entry((profile, id.to_string())).or_insert_with(Grant::new);
        grant.enabled = enabled;
        self.save()?;
        Ok(())
    }

    pub fn approve_dangerous(&mut self, profile: Uuid, id: &str, perm: Permission) -> Result<()> {
        {
            let grant = self.grants.entry((profile, id.to_string())).or_insert_with(Grant::new);
            if !grant.approved_permissions.contains(&perm) {
                return Err(ExtensionError::Invalid(
                    "сначала выдайте разрешение, затем подтвердите опасное".into(),
                ));
            }
            if !grant.dangerous_approved.contains(&perm) {
                grant.dangerous_approved.push(perm);
            }
        }
        self.save()?;
        Ok(())
    }

    pub fn revoke_profile(&mut self, profile: Uuid) {
        self.grants.retain(|(p, _), _| *p != profile);
        let _ = self.save();
    }

    pub fn delete_profile_grant(&mut self, profile: Uuid, id: &str) {
        self.grants.remove(&(profile, id.to_string()));
        let _ = self.save();
    }

    pub fn grant_for(&self, profile: Uuid, id: &str) -> Option<&Grant> {
        self.grants.get(&(profile, id.to_string()))
    }

    /// Content scripts to inject into a page (the extension runtime,
    /// §12 "content scripts by masks"): (extension_id, manifest_name,
    /// script source, approved permissions) for every enabled extension
    /// whose manifest matches `url` AND whose granted permissions actually
    /// allow reading the page (current_tab, or all_websites approved as
    /// dangerous). Scripts with no page-reading permission never run.
    pub fn content_scripts_for(&self, profile: Uuid, url: &str) -> Vec<ContentScript> {
        self.installed
            .values()
            .filter(|rec| rec.enabled_globally)
            .filter(|rec| rec.manifest.matches_url(url))
            .filter(|rec| {
                self.can(profile, &rec.manifest.id, Permission::CurrentTab)
                    || self.can(profile, &rec.manifest.id, Permission::AllWebsites)
            })
            .filter_map(|rec| {
                rec.manifest
                    .read_entry_script(&rec.install_dir)
                    .ok()
                    .map(|code| ContentScript {
                        extension_id: rec.manifest.id.clone(),
                        extension_name: rec.manifest.name.clone(),
                        all_frames: rec.manifest.all_frames,
                        code,
                        granted: rec
                            .manifest
                            .permissions
                            .iter()
                            .copied()
                            .filter(|p| self.can(profile, &rec.manifest.id, *p))
                            .collect(),
                    })
            })
            .collect()
    }

    /// Central capability check used by every extension-facing API surface.
    pub fn can(&self, profile: Uuid, id: &str, perm: Permission) -> bool {
        let Some(rec) = self.installed.get(id) else {
            return false;
        };
        if !rec.enabled_globally {
            return false;
        }
        let Some(grant) = self.grants.get(&(profile, id.to_string())) else {
            return false;
        };
        if !grant.enabled {
            return false;
        }
        grant.approved_permissions.contains(&perm)
            && (!perm.is_dangerous() || grant.dangerous_approved.contains(&perm))
    }

    /// Count extensions visible to a profile with broad access — feeds the
    /// Privacy Audit (§10A.29).
    pub fn broad_permission_count(&self, profile: Uuid) -> usize {
        self.installed
            .keys()
            .filter(|id| self.can(profile, id, Permission::AllWebsites))
            .count()
    }

    pub fn sandbox_policy(&self, profile: Uuid, id: &str) -> Result<SandboxPolicy> {
        let rec = self.installed.get(id).ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;
        let mut capabilities = Vec::new();
        for p in &rec.manifest.permissions {
            if self.can(profile, id, *p) {
                capabilities.push(p.label().to_string());
            }
        }
        Ok(SandboxPolicy {
            extension_id: id.to_string(),
            capabilities,
            denied_by_default: vec![
                "произвольное чтение файловой системы",
                "запуск бинарных файлов",
                "доступ к парольному хранилищу",
                "доступ к API-ключам",
                "чтение приватных заметок без разрешения",
                "доступ к другим профилям",
            ],
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredState {
    installed: BTreeMap<String, InstalledExtension>,
    /// JSON has string keys only, so tuple-keyed maps go out as pairs.
    grants: Vec<((Uuid, String), Grant)>,
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
// Made by MrDuck && Ox-Alpha
mod tests {
    use super::*;

    const GOOD_MANIFEST: &str = r#"{
        "id": "reading-mode",
        "name": "Reading Mode",
        "version": "1.2.0",
        "api_version": 1,
        "description": "Clean reader view",
        "permissions": ["current_tab", "selected_text"],
        "entry_point": "main.js"
    }"#;

    fn write_ext(tmp: &Path, id: &str, extra_perm: Option<&str>) -> PathBuf {
        let dir = tmp.join(format!("src-{id}"));
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        let perms = match extra_perm {
            Some(p) => format!(
                r#", "permissions": ["current_tab", "selected_text", "notes", "{p}"]"#
            ),
            None => r#", "permissions": ["current_tab", "selected_text", "notes"]"#.to_string(),
        };
        std::fs::write(
            dir.join("manifest.json"),
            format!(
                r#"{{"id":"{id}","name":"Ext {id}","version":"1.0.0","api_version":1,"entry_point":"main.js"{perms}}}"#
            ),
        )
        .unwrap();
        std::fs::write(dir.join("main.js"), "// entry").unwrap();
        std::fs::write(dir.join("assets").join("icon.svg"), "<svg/>").unwrap();
        dir
    }

    #[test]
    fn manifest_parse_and_validation() {
        let m = Manifest::parse(GOOD_MANIFEST).unwrap();
        assert_eq!(m.permissions.len(), 2);
        assert!(Manifest::parse(r#"{"id":"","name":"x","version":"1","api_version":1,"entry_point":"a"}"#).is_err());
        assert!(Manifest::parse(r#"{"id":"ok","name":"x","version":"1","api_version":99,"entry_point":"a"}"#).is_err());
        assert!(Manifest::parse(r#"{"id":"ok","name":"x","version":"1","api_version":1,"entry_point":"../../etc/passwd"}"#).is_err());
    }

    #[test]
    fn install_grant_and_capability_checks() {
        let tmp = std::env::temp_dir().join(format!("apb-ext-{}", Uuid::new_v4()));
        let src = write_ext(&tmp, "reader", None);
        let mut reg = ExtensionRegistry::open(&tmp).unwrap();

        let installed = reg.install(&src).unwrap();
        assert_eq!(installed.manifest.id, "reader");
        // Copied into managed location:
        assert!(installed.install_dir.join("assets").join("icon.svg").exists());

        let profile = Uuid::new_v4();
        // Nothing granted yet:
        assert!(!reg.can(profile, "reader", Permission::CurrentTab));

        reg.grant_permissions(profile, "reader", &[Permission::CurrentTab, Permission::SelectedText]).unwrap();
        assert!(reg.can(profile, "reader", Permission::CurrentTab));
        assert!(reg.can(profile, "reader", Permission::SelectedText));
        // Undeclared permission is refused at grant time:
        assert!(reg.grant_permissions(profile, "reader", &[Permission::Filesystem]).is_err());
        assert!(!reg.can(profile, "reader", Permission::Filesystem));

        // Disable globally kills everything:
        reg.set_enabled("reader", false).unwrap();
        assert!(!reg.can(profile, "reader", Permission::CurrentTab));
        reg.set_enabled("reader", true).unwrap();

        // Uninstall cleans disk + grants:
        reg.uninstall("reader").unwrap();
        assert!(reg.list().is_empty());
        assert!(!tmp.join("extensions/installed/reader-1.0.0").exists());
        assert!(reg.grant_for(profile, "reader").is_none());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn dangerous_permissions_need_explicit_approval() {
        let tmp = std::env::temp_dir().join(format!("apb-ext-dang-{}", Uuid::new_v4()));
        let src = write_ext(&tmp, "wide", Some("all_websites"));
        let mut reg = ExtensionRegistry::open(&tmp).unwrap();
        reg.install(&src).unwrap();

        let profile = Uuid::new_v4();
        assert!(Permission::AllWebsites.is_dangerous());
        reg.grant_permissions(profile, "wide", &[Permission::AllWebsites]).unwrap();
        // Granted but NOT confirmed => still denied:
        assert!(!reg.can(profile, "wide", Permission::AllWebsites));
        reg.approve_dangerous(profile, "wide", Permission::AllWebsites).unwrap();
        assert!(reg.can(profile, "wide", Permission::AllWebsites));
        assert_eq!(reg.broad_permission_count(profile), 1);

        let policy = reg.sandbox_policy(profile, "wide").unwrap();
        assert_eq!(policy.capabilities, vec!["Данные со всех сайтов"]);
        assert!(!policy.denied_by_default.is_empty());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn registry_state_persists_across_reopen() {
        let tmp = std::env::temp_dir().join(format!("apb-ext-persist-{}", Uuid::new_v4()));
        let src = write_ext(&tmp, "persisted", None);
        let profile = Uuid::new_v4();
        {
            let mut reg = ExtensionRegistry::open(&tmp).unwrap();
            reg.install(&src).unwrap();
            reg.set_enabled("persisted", true).unwrap();
            reg.grant_permissions(profile, "persisted", &[Permission::Notes]).unwrap();
        }
        {
            let reg = ExtensionRegistry::open(&tmp).unwrap();
            assert_eq!(reg.list().len(), 1);
            assert!(reg.can(profile, "persisted", Permission::Notes));
        }
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn duplicate_install_same_version_rejected() {
        let tmp = std::env::temp_dir().join(format!("apb-ext-dup-{}", Uuid::new_v4()));
        let src = write_ext(&tmp, "dup", None);
        let mut reg = ExtensionRegistry::open(&tmp).unwrap();
        reg.install(&src).unwrap();
        assert!(reg.install(&src).is_err());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn match_patterns_parse_and_match() {
        for good in [
            "http://*/*",
            "https://*.example.com/*",
            "*://example.com/foo*bar",
            "https://host/path/exact",
        ] {
            match_pattern::compile(good).unwrap();
        }
        for bad in [
            "ftp://example.com/*",     // scheme не поддержан
            "https://example.com",     // нет path
            "https://exa*mple.com/*",  // * в середине хоста
            "https://example.com/../x", // path traversal
            "https:// /",              // whitespace
        ] {
            assert!(match_pattern::compile(bad).is_err(), "ожидал reject: {bad}");
        }
        assert!(match_pattern::matches("http://*/*", "http://any.host/page?q=1"));
        assert!(!match_pattern::matches("http://*/*", "https://any.host/page"));
        assert!(match_pattern::matches("*://example.com/*", "https://example.com/"));
        assert!(match_pattern::matches("https://*.example.com/*", "https://a.example.com/x"));
        assert!(match_pattern::matches("https://*.example.com/*", "https://example.com/x"));
        assert!(!match_pattern::matches("https://*.example.com/*", "https://badexample.com/x"));
        assert!(!match_pattern::matches("https://*.example.com/*", "https://notexample.org/x"));
        assert!(match_pattern::matches("*://site.com/articles/*", "https://site.com/articles/2026/rust"));
        assert!(!match_pattern::matches("*://site.com/articles/*", "https://site.com/news/1"));
        assert!(match_pattern::matches("https://h.exact/path", "https://h.exact/path"));
        assert!(!match_pattern::matches("https://h.exact/path", "https://h.exact/path2"));
    }

    #[test]
    fn manifest_matches_field_validated_and_used() {
        let tmp = std::env::temp_dir().join(format!("apb-ext-match-{}", Uuid::new_v4()));
        let dir = tmp.join("src-reader");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            r#"{"id":"reader","name":"Reader","version":"1.0.0","api_version":1,
                "entry_point":"main.js","permissions":["current_tab"],
                "matches":["https://*.example.com/*"]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("main.js"), "// content script body").unwrap();
        let m = Manifest::load_from_dir(&dir).unwrap();
        assert_eq!(m.matches, vec!["https://*.example.com/*".to_string()]);
        assert!(m.matches_url("https://a.example.com/page"));
        assert!(!m.matches_url("https://other.org/page"));

        // Невалидный паттерн в matches = reject на установке.
        std::fs::write(
            dir.join("manifest.json"),
            r#"{"id":"reader","name":"Reader","version":"1.0.0","api_version":1,
                "entry_point":"main.js","permissions":["current_tab"],
                "matches":["https://*bad"]}"#,
        )
        .unwrap();
        assert!(Manifest::load_from_dir(&dir).is_err());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn content_scripts_runtime_respects_grants_and_masks() {
        let tmp = std::env::temp_dir().join(format!("apb-ext-run-{}", Uuid::new_v4()));
        let dir = tmp.join("src-runner");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            r#"{"id":"runner","name":"Runner","version":"1.0.0","api_version":1,
                "entry_point":"main.js","permissions":["current_tab"],
                "matches":["https://*.example.com/*"]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("main.js"), "console.log('ran')").unwrap();
        let mut reg = ExtensionRegistry::open(&tmp).unwrap();
        reg.install(&dir).unwrap();
        let profile = Uuid::new_v4();

        // Маска подходит, но прав нет → скрипт НЕ инжектится.
        assert!(reg.content_scripts_for(profile, "https://a.example.com/x").is_empty());

        // Право выдано → инжектится с кодом и списком прав.
        reg.grant_permissions(profile, "runner", &[Permission::CurrentTab]).unwrap();
        let scripts = reg.content_scripts_for(profile, "https://a.example.com/x");
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].extension_id, "runner");
        assert_eq!(scripts[0].code, "console.log('ran')");
        assert_eq!(scripts[0].granted, vec![Permission::CurrentTab]);

        // URL вне маски → не инжектится даже с правами.
        assert!(reg.content_scripts_for(profile, "https://elsewhere.org/x").is_empty());

        // Отключение расширения глобально глушит инжект.
        reg.set_enabled("runner", false).unwrap();
        assert!(reg.content_scripts_for(profile, "https://a.example.com/x").is_empty());
        std::fs::remove_dir_all(&tmp).ok();
    }
}

// Made by MrDuck && Ox-Alpha