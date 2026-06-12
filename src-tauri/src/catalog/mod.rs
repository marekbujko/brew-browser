//! Bundled Homebrew catalog (Phase 12a).
//!
//! The catalog is the deserialized, in-memory form of the full
//! `formulae.brew.sh` JSON dump — both formulae and casks — keyed by
//! name/token so commands can do O(1) lookups for deprecation flags,
//! descriptions, dependencies, etc., without round-tripping to brew.
//!
//! ## Source priority
//!
//! ```text
//!     resolve_active(app_data_dir)
//!         |
//!         v
//!     ┌─────────────────────────────────┐
//!     │ load_user_data(app_data_dir)    │   user-refreshed copy at
//!     │   if all three files present    │   <app_data_dir>/catalog/{...}
//!     │   and parse OK                  │
//!     └─────────────────┬───────────────┘
//!                       │ Some(c) -> return
//!                       │ None    -> fall back
//!                       │ Err     -> delete corrupt files, fall back
//!                       v
//!     ┌─────────────────────────────────┐
//!     │ load_bundled()                  │   include_bytes!d snapshot
//!     │   always succeeds in the happy  │   produced by tools/catalog/fetch.py
//!     │   path; returns an empty cat    │
//!     │   with a flag if even bundled   │
//!     │   is unparseable                │
//!     └─────────────────────────────────┘
//! ```
//!
//! ## Security caps (security-review §12a)
//!
//! - [`MAX_CATALOG_BYTES`] caps the raw HTTP response on refresh (64 MiB).
//! - [`MAX_DECOMPRESSED_BYTES`] caps the gzip output (128 MiB). The
//!   bundled snapshot is also decompressed through this cap.
//! - [`MAX_FIELD_LEN`] truncates oversized string fields post-parse so a
//!   malicious refresh can't bloat memory with one 100 MB `desc`.
//! - The catalog directory is always `app_data_dir.join("catalog")` —
//!   never composed from IPC input. See [`catalog_dir`].

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::BrewError;
use crate::util::fs::{atomic_write, read_capped};

// ---------- Security caps ----------

/// Max bytes accepted from the raw HTTP response on `catalog_refresh`.
/// Current catalog is ~30 MB formulae + ~15 MB casks raw; 64 MiB is
/// ~30% headroom for organic growth without being a memory bomb.
pub const MAX_CATALOG_BYTES: u64 = 64 * 1024 * 1024;

/// Max bytes accepted from the gzip decoder — protects against gzip
/// bombs (`Read::take` wraps the decoder). 128 MiB is 2× the raw cap so
/// honest-to-goodness compressible JSON sails through.
pub const MAX_DECOMPRESSED_BYTES: u64 = 128 * 1024 * 1024;

/// Max length for any string field after parse. Anything longer is
/// truncated by [`validate_and_truncate_formula`] /
/// [`validate_and_truncate_cask`]. 4 KiB is enormous for `desc`,
/// `homepage`, `deprecation_reason`, etc.
pub const MAX_FIELD_LEN: usize = 4096;

/// Tighter cap for `name` / `token` fields — must stay aligned with
/// `validate_package_name`'s 200-char limit.
pub const MAX_NAME_LEN: usize = 200;

// ---------- Bundled artifacts ----------

/// Bundled gzipped formulae catalog (produced by `tools/catalog/fetch.py`).
const BUNDLED_FORMULA_GZ: &[u8] = include_bytes!("../../data/catalog/formula.json.gz");

/// Bundled gzipped cask catalog.
const BUNDLED_CASK_GZ: &[u8] = include_bytes!("../../data/catalog/cask.json.gz");

/// Bundled manifest (JSON).
const BUNDLED_MANIFEST: &str = include_str!("../../data/catalog/manifest.json");

// ---------- Manifest ----------

/// The catalog manifest written by `tools/catalog/fetch.py` (bundled
/// snapshot) and by [`Catalog::write_user_data`] (user refresh).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// ISO 8601 UTC timestamp of fetch.
    pub as_of: String,
    pub formula_count: usize,
    pub cask_count: usize,
    pub formula_compressed_bytes: u64,
    pub cask_compressed_bytes: u64,
    pub fetched_from: String,
}

// ---------- Source ----------

/// Which copy of the catalog is currently active.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogSource {
    /// Baked into the binary at compile time.
    Bundled,
    /// User clicked Refresh; copy lives at
    /// `<app_data_dir>/catalog/{formula,cask}.json.gz`.
    UserRefreshed,
}

impl CatalogSource {
    /// Wire string used by `commands::catalog::CatalogSummary::source`.
    pub fn as_wire(&self) -> &'static str {
        match self {
            CatalogSource::Bundled => "bundled",
            CatalogSource::UserRefreshed => "user-refreshed",
        }
    }
}

// ---------- Formula ----------

/// Trimmed formula record — we keep only the fields the UI actually
/// surfaces today. The full upstream record has 50+ fields; reading them
/// all would bloat memory ~4× for no benefit.
///
/// `#[serde(rename_all = "camelCase")]` is for IPC output. Snake-case
/// aliases on each field accept the upstream input shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Formula {
    pub name: String,
    #[serde(alias = "full_name")]
    pub full_name: String,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    /// In the upstream JSON `license` is sometimes a structured shape
    /// (`{"any_of": [...]}` etc.) and sometimes a plain SPDX string. We
    /// flatten via a custom deserializer to a single SPDX string (or
    /// None) — see [`deserialize_license`]. The first SPDX wins.
    #[serde(default, deserialize_with = "deserialize_license")]
    pub license: Option<String>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default, alias = "deprecation_date")]
    pub deprecation_date: Option<String>,
    #[serde(default, alias = "deprecation_reason")]
    pub deprecation_reason: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default, alias = "disable_date")]
    pub disable_date: Option<String>,
    #[serde(default, alias = "disable_reason")]
    pub disable_reason: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default, alias = "build_dependencies")]
    pub build_dependencies: Vec<String>,
    #[serde(default, alias = "recommended_dependencies")]
    pub recommended_dependencies: Vec<String>,
    #[serde(default, alias = "optional_dependencies")]
    pub optional_dependencies: Vec<String>,
    #[serde(default, alias = "conflicts_with")]
    pub conflicts_with: Vec<String>,
    /// Flattened from upstream `versions: {stable, head, bottle}` —
    /// only `stable` is surfaced. See [`deserialize_versions_stable`].
    #[serde(
        default,
        rename = "versionsStable",
        alias = "versions",
        deserialize_with = "deserialize_versions_stable"
    )]
    pub versions_stable: Option<String>,
    pub tap: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

// ---------- Cask ----------

/// Trimmed cask record. The upstream cask shape differs from formulae
/// (`token` instead of `name`; `name` is an array of pretty names;
/// `version` is a single string rather than nested).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cask {
    pub token: String,
    /// Pretty display name(s) — casks routinely have multiple. The first
    /// is the canonical one; the rest are alternative spellings.
    #[serde(default)]
    pub name: Vec<String>,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default, alias = "deprecation_date")]
    pub deprecation_date: Option<String>,
    #[serde(default, alias = "deprecation_reason")]
    pub deprecation_reason: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    pub version: Option<String>,
    pub tap: String,
    /// Formulae this cask requires, plucked from the nested upstream
    /// `depends_on.formula` array (e.g. cask `aptible` →
    /// `["libfido2"]`). Used to surface a formula's *cask* reverse-
    /// dependents — the macOS-only superset of the reverse-deps graph.
    /// Empty for the vast majority of casks. See
    /// [`deserialize_depends_on_formula`].
    #[serde(
        default,
        rename = "dependsOnFormula",
        alias = "depends_on",
        deserialize_with = "deserialize_depends_on_formula"
    )]
    pub depends_on_formula: Vec<String>,
}

// ---------- Custom deserializers ----------

/// Accept either:
/// - `null`
/// - a plain string (`"MIT"`, `"GPL-3.0-or-later"`)
/// - a structured object (`{"any_of": ["MIT", "Apache-2.0"]}`,
///   `{"all_of": [...]}`, etc.)
///
/// and produce `Option<String>`. For structured shapes we pluck the
/// first SPDX-looking leaf string. Catalogs in the wild today are
/// virtually all plain strings, but the API has emitted structured
/// shapes historically — better to handle defensively now than fail
/// the whole parse later.
fn deserialize_license<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(d)?;
    let Some(v) = v else { return Ok(None) };
    fn first_string(v: &serde_json::Value) -> Option<String> {
        match v {
            serde_json::Value::String(s) => {
                if s.trim().is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }
            serde_json::Value::Array(arr) => arr.iter().find_map(first_string),
            serde_json::Value::Object(map) => {
                // `{"any_of": [...]}`, `{"all_of": [...]}` and similar.
                map.values().find_map(first_string)
            }
            _ => None,
        }
    }
    if v.is_null() {
        return Ok(None);
    }
    // Unknown shapes (numbers, bools) silently degrade to None — better
    // than failing the whole catalog parse for a single weird record.
    Ok(first_string(&v))
}

/// Accept the upstream `versions: { stable, head, bottle }` object and
/// produce just the `stable` string (or None when null/missing).
fn deserialize_versions_stable<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(d)?;
    let Some(v) = v else { return Ok(None) };
    if v.is_null() {
        return Ok(None);
    }
    // Either it's the full object…
    if let Some(obj) = v.as_object() {
        if let Some(serde_json::Value::String(s)) = obj.get("stable") {
            return Ok(Some(s.clone()));
        }
        return Ok(None);
    }
    // …or someone passed a bare string (back-compat with hand-rolled fixtures).
    if let Some(s) = v.as_str() {
        return Ok(Some(s.to_string()));
    }
    Ok(None)
}

/// Pluck `depends_on.formula` out of the upstream cask `depends_on`
/// object and produce a flat `Vec<String>` of formula tokens.
///
/// Upstream shape is a nested object, e.g.
/// `{"macos": {...}, "formula": ["libfido2"]}`. The `formula` value is
/// an array of bare formula tokens in this dataset; a few historical
/// records have surfaced it as a single bare string, so we accept both.
/// Missing / null `depends_on`, or a `depends_on` with no `formula`
/// key, yields an empty vec. Any unexpected shape (number, bool)
/// silently degrades to empty rather than failing the whole cask parse.
fn deserialize_depends_on_formula<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(d)?;
    let Some(v) = v else { return Ok(Vec::new()) };
    let Some(obj) = v.as_object() else {
        return Ok(Vec::new());
    };
    let Some(formula) = obj.get("formula") else {
        return Ok(Vec::new());
    };
    match formula {
        serde_json::Value::String(s) => {
            if s.trim().is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![s.clone()])
            }
        }
        serde_json::Value::Array(arr) => Ok(arr
            .iter()
            .filter_map(|e| e.as_str().map(|s| s.to_string()))
            .collect()),
        _ => Ok(Vec::new()),
    }
}

// ---------- Catalog ----------

/// Full in-memory catalog. Backed by hash maps so commands can do O(1)
/// lookups by name/token.
#[derive(Debug, Clone)]
pub struct Catalog {
    pub formulae: HashMap<String, Formula>,
    pub casks: HashMap<String, Cask>,
    pub as_of: String,
    pub source: CatalogSource,
    pub formula_count: usize,
    pub cask_count: usize,
    /// True iff we couldn't parse either bundled or user-refreshed data.
    /// Surfaced so the UI can render a "catalog unavailable" hint.
    /// Empty catalogs from this flag still resolve `Some(..)` from
    /// [`Catalog::resolve_active`] — the alternative would be every
    /// caller having to handle "no catalog" specifically.
    pub corrupt: bool,
}

impl Catalog {
    /// Load the bundled snapshot (always present — it's `include_bytes!`d).
    ///
    /// Decompresses through a `Read::take(MAX_DECOMPRESSED_BYTES)`
    /// wrapper so the gzip-bomb cap applies even to data we built
    /// ourselves (defense in depth — keeps the helper honest if someone
    /// later swaps the bundled bytes for a third-party feed).
    pub fn load_bundled() -> Result<Catalog, BrewError> {
        let manifest: Manifest = serde_json::from_str(BUNDLED_MANIFEST).map_err(|e| {
            BrewError::Internal {
                message: format!("bundled manifest parse failed: {e}"),
            }
        })?;

        let formulae_bytes = decompress_capped(BUNDLED_FORMULA_GZ, "bundled formula.json.gz")?;
        let casks_bytes = decompress_capped(BUNDLED_CASK_GZ, "bundled cask.json.gz")?;

        let formulae = parse_formulae(&formulae_bytes)?;
        let casks = parse_casks(&casks_bytes)?;

        Ok(Catalog {
            formula_count: formulae.len(),
            cask_count: casks.len(),
            formulae,
            casks,
            as_of: manifest.as_of,
            source: CatalogSource::Bundled,
            corrupt: false,
        })
    }

    /// Load the user-refreshed copy from
    /// `<app_data_dir>/catalog/{formula,cask}.json.gz` + `manifest.json`.
    ///
    /// Returns:
    /// - `Ok(None)` if any of the three files is missing — this is the
    ///   first-launch / never-refreshed case.
    /// - `Ok(Some(c))` if all three load and parse cleanly.
    /// - `Err(_)` if any file is present but unreadable / unparseable
    ///   / oversize. Callers should treat this as "delete the user-data
    ///   files and fall back to bundled".
    pub async fn load_user_data(app_data_dir: &Path) -> Result<Option<Catalog>, BrewError> {
        let dir = catalog_dir(app_data_dir);
        let formula_path = dir.join("formula.json.gz");
        let cask_path = dir.join("cask.json.gz");
        let manifest_path = dir.join("manifest.json");

        // First-launch / pristine case — at least one of the three is
        // missing, so we have no user-refreshed copy.
        if !formula_path.exists() || !cask_path.exists() || !manifest_path.exists() {
            return Ok(None);
        }

        // Read all three with size caps. Anything bigger than the cap
        // is treated as corruption (Err), not silent truncation.
        let manifest_bytes = read_capped(&manifest_path, 1024 * 1024).await?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes).map_err(|e| {
            BrewError::JsonParse {
                command: "load_user_data manifest.json".into(),
                message: e.to_string(),
                raw_excerpt: String::new(),
            }
        })?;

        let formula_gz = read_capped(&formula_path, MAX_CATALOG_BYTES).await?;
        let cask_gz = read_capped(&cask_path, MAX_CATALOG_BYTES).await?;

        let formulae_bytes = decompress_capped(&formula_gz, "user-data formula.json.gz")?;
        let casks_bytes = decompress_capped(&cask_gz, "user-data cask.json.gz")?;

        let formulae = parse_formulae(&formulae_bytes)?;
        let casks = parse_casks(&casks_bytes)?;

        Ok(Some(Catalog {
            formula_count: formulae.len(),
            cask_count: casks.len(),
            formulae,
            casks,
            as_of: manifest.as_of,
            source: CatalogSource::UserRefreshed,
            corrupt: false,
        }))
    }

    /// Resolve the active catalog: user-refreshed if present + parseable,
    /// else bundled. Corrupt user-data files are deleted (so a one-time
    /// bad refresh doesn't pin the app to a broken state) and bundled
    /// is returned. NEVER panics — even total failure produces an empty
    /// catalog with `corrupt: true`.
    pub async fn resolve_active(app_data_dir: &Path) -> Catalog {
        match Catalog::load_user_data(app_data_dir).await {
            Ok(Some(c)) => return c,
            Ok(None) => {
                // Pristine case — fall through to bundled.
            }
            Err(e) => {
                // Corrupt / oversize user-data → log + cleanup + bundled.
                tracing::warn!(
                    "catalog: user-data unreadable ({}); deleting and falling back to bundled",
                    e
                );
                let dir = catalog_dir(app_data_dir);
                let _ = tokio::fs::remove_file(dir.join("formula.json.gz")).await;
                let _ = tokio::fs::remove_file(dir.join("cask.json.gz")).await;
                let _ = tokio::fs::remove_file(dir.join("manifest.json")).await;
            }
        }
        match Catalog::load_bundled() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    "catalog: bundled snapshot failed to parse ({}); serving empty catalog",
                    e
                );
                Catalog {
                    formulae: HashMap::new(),
                    casks: HashMap::new(),
                    as_of: String::new(),
                    source: CatalogSource::Bundled,
                    formula_count: 0,
                    cask_count: 0,
                    corrupt: true,
                }
            }
        }
    }

    /// Atomically write the user-refreshed catalog files. Caller is
    /// responsible for ensuring `formulae_bytes` and `casks_bytes` are
    /// already gzip-compressed and that `manifest` describes them
    /// accurately. The catalog directory will be created if missing.
    pub async fn write_user_data(
        app_data_dir: &Path,
        formulae_bytes: &[u8],
        casks_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<(), BrewError> {
        let dir = catalog_dir(app_data_dir);
        if !dir.exists() {
            tokio::fs::create_dir_all(&dir).await.map_err(|e| BrewError::Io {
                message: format!("create catalog dir {}: {}", dir.display(), e),
            })?;
        }
        atomic_write(&dir.join("formula.json.gz"), formulae_bytes).await?;
        atomic_write(&dir.join("cask.json.gz"), casks_bytes).await?;
        let manifest_bytes = serde_json::to_vec_pretty(manifest).map_err(|e| BrewError::Io {
            message: format!("serialize manifest: {e}"),
        })?;
        atomic_write(&dir.join("manifest.json"), &manifest_bytes).await?;
        Ok(())
    }
}

// ---------- Helpers ----------

/// The catalog directory on disk. ALWAYS `app_data_dir.join("catalog")` —
/// never composed from IPC input. This is the single helper called out
/// by the Phase 12 security review §12a.
pub fn catalog_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("catalog")
}

/// Decompress gzipped `bytes` with a cap of [`MAX_DECOMPRESSED_BYTES`]
/// applied via `Read::take`. Anything beyond the cap returns
/// `BrewError::Io` describing where it overflowed.
fn decompress_capped(bytes: &[u8], context: &str) -> Result<Vec<u8>, BrewError> {
    let mut decoder = flate2::read::GzDecoder::new(bytes).take(MAX_DECOMPRESSED_BYTES + 1);
    let mut buf = Vec::with_capacity(bytes.len() * 4);
    decoder.read_to_end(&mut buf).map_err(|e| BrewError::Io {
        message: format!("decompress {}: {}", context, e),
    })?;
    if buf.len() as u64 > MAX_DECOMPRESSED_BYTES {
        return Err(BrewError::Io {
            message: format!(
                "decompressed {} exceeds {} byte cap",
                context, MAX_DECOMPRESSED_BYTES
            ),
        });
    }
    Ok(buf)
}

fn parse_formulae(bytes: &[u8]) -> Result<HashMap<String, Formula>, BrewError> {
    let arr: Vec<Formula> = serde_json::from_slice(bytes).map_err(|e| BrewError::JsonParse {
        command: "catalog formula.json".into(),
        message: e.to_string(),
        raw_excerpt: String::new(),
    })?;
    let mut map = HashMap::with_capacity(arr.len());
    for mut f in arr {
        validate_and_truncate_formula(&mut f);
        if f.name.is_empty() {
            continue;
        }
        map.insert(f.name.clone(), f);
    }
    Ok(map)
}

fn parse_casks(bytes: &[u8]) -> Result<HashMap<String, Cask>, BrewError> {
    let arr: Vec<Cask> = serde_json::from_slice(bytes).map_err(|e| BrewError::JsonParse {
        command: "catalog cask.json".into(),
        message: e.to_string(),
        raw_excerpt: String::new(),
    })?;
    let mut map = HashMap::with_capacity(arr.len());
    for mut c in arr {
        validate_and_truncate_cask(&mut c);
        if c.token.is_empty() {
            continue;
        }
        map.insert(c.token.clone(), c);
    }
    Ok(map)
}

/// Post-parse field-length cap. Truncates rather than rejects so a
/// single oversized field doesn't blow away the rest of the record.
fn validate_and_truncate_formula(f: &mut Formula) {
    truncate_in_place(&mut f.name, MAX_NAME_LEN);
    truncate_in_place(&mut f.full_name, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.desc, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.homepage, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.license, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.deprecation_date, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.deprecation_reason, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.disable_date, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.disable_reason, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut f.versions_stable, MAX_FIELD_LEN);
    truncate_in_place(&mut f.tap, MAX_FIELD_LEN);
    truncate_vec_in_place(&mut f.dependencies, MAX_NAME_LEN);
    truncate_vec_in_place(&mut f.build_dependencies, MAX_NAME_LEN);
    truncate_vec_in_place(&mut f.recommended_dependencies, MAX_NAME_LEN);
    truncate_vec_in_place(&mut f.optional_dependencies, MAX_NAME_LEN);
    truncate_vec_in_place(&mut f.conflicts_with, MAX_NAME_LEN);
    truncate_vec_in_place(&mut f.aliases, MAX_NAME_LEN);
}

fn validate_and_truncate_cask(c: &mut Cask) {
    truncate_in_place(&mut c.token, MAX_NAME_LEN);
    truncate_vec_in_place(&mut c.name, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut c.desc, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut c.homepage, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut c.deprecation_date, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut c.deprecation_reason, MAX_FIELD_LEN);
    truncate_opt_in_place(&mut c.version, MAX_FIELD_LEN);
    truncate_in_place(&mut c.tap, MAX_FIELD_LEN);
    truncate_vec_in_place(&mut c.depends_on_formula, MAX_NAME_LEN);
}

fn truncate_in_place(s: &mut String, max: usize) {
    if s.len() <= max {
        return;
    }
    // Walk back to a UTF-8 char boundary.
    let mut idx = max;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    s.truncate(idx);
}

fn truncate_opt_in_place(s: &mut Option<String>, max: usize) {
    if let Some(inner) = s.as_mut() {
        truncate_in_place(inner, max);
    }
}

fn truncate_vec_in_place(v: &mut [String], max: usize) {
    for s in v.iter_mut() {
        truncate_in_place(s, max);
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses() {
        let m: Manifest =
            serde_json::from_str(BUNDLED_MANIFEST).expect("bundled manifest parses");
        assert!(!m.as_of.is_empty());
        assert!(m.formula_count > 1000);
        assert!(m.cask_count > 1000);
    }

    #[test]
    fn decompress_capped_round_trips() {
        // Compress + decompress the bundled formula bytes — happy path
        // through the helper.
        let out = decompress_capped(BUNDLED_FORMULA_GZ, "bundled formula").expect("decompress");
        // Decompressed formula payload is ~30 MB.
        assert!(out.len() > 10 * 1024 * 1024);
    }

    #[test]
    fn deserialize_license_accepts_plain_string() {
        let json = serde_json::json!("MIT");
        let lic: Option<String> = serde_json::from_value(json).unwrap();
        // Direct deserialization without the custom hook still works
        // (sanity check on serde's default). Below we test the hook.
        assert_eq!(lic.as_deref(), Some("MIT"));
    }

    #[test]
    fn deserialize_license_via_hook_handles_object_shape() {
        // Construct a tiny formula JSON with structured license and
        // verify our custom deserializer flattens it.
        let raw = r#"{
            "name": "foo",
            "full_name": "foo",
            "desc": null,
            "homepage": null,
            "license": {"any_of": ["MIT", "Apache-2.0"]},
            "deprecated": false,
            "disabled": false,
            "dependencies": [],
            "recommended_dependencies": [],
            "optional_dependencies": [],
            "conflicts_with": [],
            "versions": {"stable": "1.0.0", "head": null, "bottle": true},
            "tap": "homebrew/core",
            "aliases": []
        }"#;
        let f: Formula = serde_json::from_str(raw).expect("parse");
        assert_eq!(f.license.as_deref(), Some("MIT"));
        assert_eq!(f.versions_stable.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn deserialize_license_handles_null() {
        let raw = r#"{
            "name": "foo",
            "full_name": "foo",
            "license": null,
            "tap": "homebrew/core"
        }"#;
        let f: Formula = serde_json::from_str(raw).expect("parse");
        assert!(f.license.is_none());
    }

    #[test]
    fn truncate_in_place_respects_utf8_boundaries() {
        let mut s = "日本語日本語日本語".to_string(); // 3-byte chars
        truncate_in_place(&mut s, 5);
        // Must not panic; result is valid utf-8.
        assert!(s.len() <= 5);
        assert!(s.chars().all(|c| c.is_ascii() || c == '日' || c == '本' || c == '語'));
    }

    #[test]
    fn truncate_in_place_no_op_when_under_cap() {
        let mut s = "abc".to_string();
        truncate_in_place(&mut s, 10);
        assert_eq!(s, "abc");
    }

    #[test]
    fn catalog_dir_is_app_data_subdir() {
        let root = std::path::Path::new("/tmp/brew-browser");
        let d = catalog_dir(root);
        assert_eq!(d, std::path::PathBuf::from("/tmp/brew-browser/catalog"));
    }

    #[test]
    fn catalog_source_wire_strings() {
        assert_eq!(CatalogSource::Bundled.as_wire(), "bundled");
        assert_eq!(CatalogSource::UserRefreshed.as_wire(), "user-refreshed");
    }

    #[tokio::test]
    async fn load_bundled_succeeds_and_populates_maps() {
        let cat = Catalog::load_bundled().expect("load bundled");
        assert!(cat.formulae.len() > 1000, "expected >1k formulae");
        assert!(cat.casks.len() > 1000, "expected >1k casks");
        assert_eq!(cat.source, CatalogSource::Bundled);
        assert!(!cat.as_of.is_empty());
        assert!(!cat.corrupt);
    }

    #[tokio::test]
    async fn load_user_data_returns_none_when_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = Catalog::load_user_data(tmp.path()).await.expect("load");
        assert!(r.is_none(), "no user-data on a fresh dir");
    }

    #[tokio::test]
    async fn resolve_active_falls_back_to_bundled() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cat = Catalog::resolve_active(tmp.path()).await;
        assert_eq!(cat.source, CatalogSource::Bundled);
        assert!(cat.formulae.len() > 1000);
    }

    #[tokio::test]
    async fn write_user_data_then_load_round_trips() {
        let tmp = tempfile::tempdir().expect("tempdir");

        // Re-use the bundled bytes as the "user-refreshed" payload — it's
        // a valid, parseable catalog so we can confirm the full round
        // trip without a fresh network fetch.
        let manifest = Manifest {
            as_of: "2026-05-24T12:00:00Z".to_string(),
            formula_count: 0,
            cask_count: 0,
            formula_compressed_bytes: BUNDLED_FORMULA_GZ.len() as u64,
            cask_compressed_bytes: BUNDLED_CASK_GZ.len() as u64,
            fetched_from: "https://formulae.brew.sh/api/".to_string(),
        };
        Catalog::write_user_data(tmp.path(), BUNDLED_FORMULA_GZ, BUNDLED_CASK_GZ, &manifest)
            .await
            .expect("write_user_data");

        let loaded = Catalog::load_user_data(tmp.path())
            .await
            .expect("load_user_data")
            .expect("Some(catalog)");
        assert_eq!(loaded.source, CatalogSource::UserRefreshed);
        assert_eq!(loaded.as_of, "2026-05-24T12:00:00Z");
        assert!(loaded.formulae.len() > 1000);
    }

    #[tokio::test]
    async fn resolve_active_recovers_from_corrupt_user_data() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = catalog_dir(tmp.path());
        tokio::fs::create_dir_all(&dir).await.unwrap();
        // Plant three corrupt files (all three required for load_user_data
        // to even attempt a parse).
        tokio::fs::write(dir.join("formula.json.gz"), b"not a gzip stream")
            .await
            .unwrap();
        tokio::fs::write(dir.join("cask.json.gz"), b"not a gzip stream")
            .await
            .unwrap();
        tokio::fs::write(dir.join("manifest.json"), b"{")
            .await
            .unwrap();

        let cat = Catalog::resolve_active(tmp.path()).await;
        // Must have fallen back to bundled.
        assert_eq!(cat.source, CatalogSource::Bundled);
        assert!(cat.formulae.len() > 1000);
        // …and cleaned up the corrupt files.
        assert!(!dir.join("formula.json.gz").exists());
        assert!(!dir.join("cask.json.gz").exists());
        assert!(!dir.join("manifest.json").exists());
    }

    #[test]
    fn deprecation_flag_surfaces_on_parsed_formula() {
        // Pull a known-deprecated formula from the bundled snapshot. The
        // catalog naturally has hundreds, so any single one will do as
        // long as we don't pin to a specific name (those churn).
        let cat = Catalog::load_bundled().expect("load bundled");
        let dep = cat.formulae.values().find(|f| f.deprecated).expect(
            "bundled catalog should contain at least one deprecated formula",
        );
        assert!(dep.deprecated);
        // Most deprecated formulae carry a reason — but it's not required,
        // so just check the flag plumbed through.
    }

    #[test]
    fn known_formula_wget_present_in_bundled() {
        let cat = Catalog::load_bundled().expect("load bundled");
        let wget = cat.formulae.get("wget").expect("wget must be in catalog");
        assert_eq!(wget.name, "wget");
        assert!(wget.versions_stable.is_some());
    }
}
