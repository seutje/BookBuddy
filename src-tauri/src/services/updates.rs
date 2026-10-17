//! Fixed release origin, signed installer, explicit consent and recovery copy.
use crate::{domain::AppError, services::backups};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::time::Duration;
use tauri::{ipc::Channel, AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

const MAX_DOWNLOAD: usize = 128 * 1024 * 1024;
#[derive(Default)]
pub struct UpdateState {
    pending: Mutex<Option<Update>>,
    busy: AtomicBool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    pub notes: String,
}
#[derive(Serialize)]
pub struct Check {
    pub enabled: bool,
    pub update: Option<Release>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: &'static str,
    pub downloaded: usize,
    pub total: Option<u64>,
    pub recovery_path: Option<String>,
}
fn failure() -> AppError {
    AppError {
        code: crate::domain::ErrorCode::Validation,
        message: "The update could not be checked, verified or started. Your household data has not been replaced. Check your connection and try again.".into(),
    }
}
struct Busy<'a>(&'a AtomicBool);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl UpdateState {
    fn enter(&self) -> Result<Busy<'_>, AppError> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| AppError::validation("An update operation is already running or the reviewed version changed. Check again."))?;
        Ok(Busy(&self.busy))
    }
}
pub fn allowed_transport(url: &tauri::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.fragment().is_none()
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
pub fn allowed_artifact(url: &tauri::Url) -> bool {
    allowed_transport(url)
        && url.host_str() == Some("github.com")
        && url
            .path()
            .starts_with("/seutje/BookBuddy/releases/download/")
        && url.path().ends_with(".exe")
        && url.query().is_none()
}
fn release(update: &Update) -> Result<Release, AppError> {
    if !allowed_artifact(&update.download_url)
        || update.raw_json.to_string().len() > 1024 * 1024
        || update.body.as_ref().is_some_and(|s| s.len() > 16_000)
    {
        return Err(failure());
    }
    Ok(Release {
        version: update.version.clone(),
        notes: update.body.clone().unwrap_or_default(),
    })
}
pub async fn check<R: Runtime>(app: &AppHandle<R>, state: &UpdateState) -> Result<Check, AppError> {
    // Development and acceptance examples do not contact a production feed.
    if cfg!(debug_assertions) || std::env::args().any(|arg| arg.starts_with("--acceptance-run=")) {
        return Ok(Check {
            enabled: false,
            update: None,
        });
    }
    let _busy = state.enter()?;
    *state.pending.lock().map_err(|_| failure())? = None;
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(15))
        .no_proxy()
        .configure_client(|client| {
            client.redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() < 5 && allowed_transport(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
        })
        .build()
        .map_err(|_| failure())?;
    let update = updater.check().await.map_err(|_| failure())?;
    let description = update.as_ref().map(release).transpose()?;
    *state.pending.lock().map_err(|_| failure())? = update;
    Ok(Check {
        enabled: true,
        update: description,
    })
}
pub async fn install(
    state: &UpdateState,
    pool: &sqlx::SqlitePool,
    directory: &std::path::Path,
    version: &str,
    confirmed: bool,
    progress: Channel<Progress>,
) -> Result<(), AppError> {
    if !confirmed {
        return Err(AppError::validation(
            "Confirm downloading and installing this update first.",
        ));
    }
    let _busy = state.enter()?;
    let mut update = state
        .pending
        .lock()
        .map_err(|_| failure())?
        .clone()
        .ok_or_else(failure)?;
    if update.version != version {
        return Err(AppError::validation(
            "The reviewed update changed. Check for updates again.",
        ));
    }
    release(&update)?;
    update.timeout = Some(Duration::from_secs(120));
    let exceeded = tokio::sync::Notify::new();
    let mut downloaded = 0usize;
    let download = update.download(
        |chunk, total| {
            downloaded = downloaded.saturating_add(chunk);
            if downloaded > MAX_DOWNLOAD || total.is_some_and(|n| n > MAX_DOWNLOAD as u64) {
                exceeded.notify_one();
            }
            let _ = progress.send(Progress {
                phase: "downloading",
                downloaded,
                total,
                recovery_path: None,
            });
        },
        || {},
    );
    let bytes = tokio::select! {
        biased;
        _=exceeded.notified()=>return Err(failure()),
        result=download=>result.map_err(|_|failure())?,
    };
    if bytes.len() > MAX_DOWNLOAD {
        return Err(failure());
    }
    let _ = progress.send(Progress {
        phase: "backing_up",
        downloaded: bytes.len(),
        total: Some(bytes.len() as u64),
        recovery_path: None,
    });
    let recovery = recovery_backup(pool, directory).await?;
    let _ = progress.send(Progress {
        phase: "installing",
        downloaded: bytes.len(),
        total: Some(bytes.len() as u64),
        recovery_path: Some(recovery),
    });
    // The SDK verifies bytes before this point and exits only after launching
    // the NSIS installer. SQLite commits/WAL remain durable if handoff fails.
    tauri::async_runtime::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|_| failure())?
        .map_err(|_| failure())
}
pub async fn recovery_backup(
    pool: &sqlx::SqlitePool,
    directory: &std::path::Path,
) -> Result<String, AppError> {
    let path = directory
        .join("backups")
        .join(format!("pre-update-{}.bookbuddy", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|_| failure())?;
    backups::export(pool, directory, &path).await?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_fixed_https_release_artifacts_and_cdn_redirects_are_allowed() {
        for u in [
            "http://github.com/seutje/BookBuddy/releases/download/v1/a.exe",
            "https://127.0.0.1/a.exe",
            "https://github.com.evil.com/a.exe",
            "https://user:secret@github.com/seutje/BookBuddy/releases/download/v1/a.exe",
            "https://github.com/seutje/WorDrop/releases/download/v1/a.exe",
            "https://github.com/seutje/BookBuddy/releases/download/v1/a.zip",
            "https://github.com/seutje/BookBuddy/releases/download/v1/a.exe?secret=1",
        ] {
            assert!(!allowed_artifact(&u.parse().unwrap()));
        }
        assert!(allowed_artifact(&"https://github.com/seutje/BookBuddy/releases/download/v0.2.0/BookBuddy_0.2.0_x64-setup.exe".parse().unwrap()));
        assert!(allowed_transport(
            &"https://release-assets.githubusercontent.com/owned?signature=opaque"
                .parse()
                .unwrap()
        ));
        let state = UpdateState::default();
        let permit = state.enter().unwrap();
        assert!(state.enter().is_err());
        drop(permit);
        assert!(state.enter().is_ok());
    }
}
