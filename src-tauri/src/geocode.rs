//! Place search via Nominatim (OpenStreetMap), following its usage policy:
//! - identifying User-Agent (app name, repo URL, optional contact email)
//! - at most 1 request per second across the whole app
//! - searches only on explicit user action (the UI calls this on Enter/Find)
//! - results cached on disk so repeat searches never hit the server

use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const APP_ID: &str = concat!("bulk-geotag/", env!("CARGO_PKG_VERSION"));
const REPO_URL: &str = "https://github.com/ZuluPilgrim/bulk-geotag";
const MIN_INTERVAL: Duration = Duration::from_millis(1100); // a little over 1 s, to stay safely under the limit

/// One search result, as shown in the UI list and on the map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub label: String,
    pub lat: f64,
    pub lon: f64,
}

/// Raw Nominatim result — lat/lon arrive as strings.
#[derive(Deserialize)]
struct NominatimPlace {
    lat: String,
    lon: String,
    display_name: String,
}

/// Shared Nominatim client, kept in Tauri state for the life of the app.
/// Holds the HTTP client, the app-wide rate limiter and the on-disk result cache.
pub struct Geocoder {
    client: reqwest::Client,
    last_request: Mutex<Option<Instant>>,
    cache: Mutex<Option<HashMap<String, Vec<Candidate>>>>, // loaded lazily from disk
    cache_path: PathBuf,
}

impl Geocoder {
    /// `cache_path`: JSON file in the app cache folder. It is read lazily on first use.
    pub fn new(cache_path: PathBuf) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("HTTP client"),
            last_request: Mutex::new(None),
            cache: Mutex::new(None),
            cache_path,
        }
    }

    /// Identifying User-Agent required by the usage policy: app/version, repo URL, optional email.
    fn user_agent(settings: &Settings) -> String {
        let email = settings.contact_email.trim();
        if email.is_empty() {
            format!("{APP_ID} (+{REPO_URL})")
        } else {
            format!("{APP_ID} (+{REPO_URL}; {email})")
        }
    }

    /// Cache key: server + language + limit + query (lower-cased, whitespace collapsed),
    /// so "Rome  ITALY" and "rome italy" share one entry, and changing the server, language or
    /// result count in Settings never reuses old results.
    fn cache_key(settings: &Settings, query: &str) -> String {
        let q = query.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
        format!(
            "{}|{}|{}|{q}",
            settings.nominatim_url.trim_end_matches('/'),
            settings.language,
            settings.max_results
        )
    }

    /// Number of searches in the cache (for the Settings dialog).
    pub fn cache_len(&self) -> usize {
        self.with_cache(|m| m.len())
    }

    /// Empty the cache in memory and on disk. Returns how many searches were removed.
    pub fn clear_cache(&self) -> usize {
        let removed = self.with_cache(|m| {
            let n = m.len();
            m.clear();
            n
        });
        let _ = fs::remove_file(&self.cache_path);
        removed
    }

    /// Run `f` on the cache, loading it from disk the first time. A missing or broken file means an empty cache.
    fn with_cache<R>(&self, f: impl FnOnce(&mut HashMap<String, Vec<Candidate>>) -> R) -> R {
        let mut guard = self.cache.lock().unwrap();
        let map = guard.get_or_insert_with(|| {
            fs::read_to_string(&self.cache_path)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default()
        });
        f(map)
    }

    /// Write the cache to disk. Best-effort: failure only means the next launch searches again.
    fn save_cache(&self) {
        let json = self.with_cache(|m| serde_json::to_string(m).unwrap_or_default());
        if let Some(dir) = self.cache_path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&self.cache_path, json); // cache is best-effort
    }

    /// How long to wait before the next request is allowed. Reserves the slot.
    fn reserve_slot(&self) -> Duration {
        let mut last = self.last_request.lock().unwrap();
        let now = Instant::now();
        let next_allowed = last.map_or(now, |t| (t + MIN_INTERVAL).max(now));
        *last = Some(next_allowed);
        next_allowed - now
    }

    /// Look up `query`: cache first; otherwise wait for a rate-limit slot, call `/search`,
    /// turn HTTP errors into readable messages, and cache the parsed results.
    pub async fn search(&self, settings: &Settings, query: &str) -> Result<Vec<Candidate>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let key = Self::cache_key(settings, query);
        if let Some(hit) = self.with_cache(|m| m.get(&key).cloned()) {
            return Ok(hit);
        }

        let wait = self.reserve_slot();
        if !wait.is_zero() {
            tauri::async_runtime::spawn_blocking(move || std::thread::sleep(wait))
                .await
                .map_err(|e| e.to_string())?;
        }

        let url = format!("{}/search", settings.nominatim_url.trim_end_matches('/'));
        let mut params = vec![
            ("q", query.to_string()),
            ("format", "jsonv2".to_string()),
            ("limit", settings.max_results.clamp(1, 10).to_string()),
            ("accept-language", settings.language.clone()),
        ];
        let email = settings.contact_email.trim();
        if !email.is_empty() {
            params.push(("email", email.to_string()));
        }

        let response = self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, Self::user_agent(settings))
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("could not reach {url}: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(match status.as_u16() {
                403 => "Nominatim refused the request (403). Check the contact email in ⚙ Settings, or try again later.".into(),
                429 => "Nominatim says too many requests (429). Wait a minute and try again.".into(),
                code => format!("Nominatim returned HTTP {code}"),
            });
        }

        let places: Vec<NominatimPlace> = response
            .json()
            .await
            .map_err(|e| format!("unexpected reply from Nominatim: {e}"))?;
        let candidates = parse_places(places);

        self.with_cache(|m| m.insert(key, candidates.clone()));
        self.save_cache();
        Ok(candidates)
    }
}

/// Convert Nominatim rows to candidates, dropping any whose lat/lon don't parse.
fn parse_places(places: Vec<NominatimPlace>) -> Vec<Candidate> {
    places
        .into_iter()
        .filter_map(|p| {
            Some(Candidate {
                lat: p.lat.parse().ok()?,
                lon: p.lon.parse().ok()?,
                label: p.display_name,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nominatim_json() {
        let raw = r#"[{"place_id":1,"lat":"41.8902102","lon":"12.4922309","display_name":"Colosseum, Rome, Italy","type":"attraction"},
                      {"lat":"bad","lon":"1","display_name":"broken"}]"#;
        let places: Vec<NominatimPlace> = serde_json::from_str(raw).unwrap();
        let c = parse_places(places);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].label, "Colosseum, Rome, Italy");
        assert!((c[0].lat - 41.8902102).abs() < 1e-9);
    }

    #[test]
    fn cache_key_normalises_query() {
        let s = Settings::default();
        assert_eq!(Geocoder::cache_key(&s, "  Rome   ITALY "), Geocoder::cache_key(&s, "rome italy"));
    }

    #[test]
    fn cache_key_includes_language_and_limit() {
        let a = Settings::default();
        let b = Settings { language: "af".into(), ..Default::default() };
        let c = Settings { max_results: 3, ..Default::default() };
        let k = |s: &Settings| Geocoder::cache_key(s, "rome");
        assert_ne!(k(&a), k(&b));
        assert_ne!(k(&a), k(&c));
    }

    #[test]
    fn clear_cache_empties_it() {
        let path = std::env::temp_dir().join(format!("bg-cache-{}.json", std::process::id()));
        let g = Geocoder::new(path.clone());
        g.with_cache(|m| m.insert("k".into(), vec![]));
        g.save_cache();
        assert_eq!(g.cache_len(), 1);
        assert_eq!(g.clear_cache(), 1);
        assert_eq!(g.cache_len(), 0);
        assert!(!path.exists());
    }

    #[test]
    fn user_agent_includes_email_when_set() {
        let mut s = Settings::default();
        assert!(!Geocoder::user_agent(&s).contains(';'));
        s.contact_email = "me@example.com".into();
        assert!(Geocoder::user_agent(&s).ends_with("; me@example.com)"));
    }

    #[test]
    fn rate_limit_spaces_requests() {
        let g = Geocoder::new(std::env::temp_dir().join("bg-test-cache.json"));
        assert!(g.reserve_slot().is_zero());
        assert!(g.reserve_slot() >= Duration::from_millis(1000));
        assert!(g.reserve_slot() >= Duration::from_millis(2100)); // queued behind the previous one
    }
}
