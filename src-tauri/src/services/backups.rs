use crate::{
    domain::{backups::*, AppError},
    persistence::{self, backups as repository, snapshots},
    services::xlsx_reader::hash,
};
use sqlx::SqlitePool;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const MAX_ARCHIVE: u64 = 256 * 1024 * 1024;
const MAX_DATABASE: u64 = 512 * 1024 * 1024;
fn failure(message: &str) -> AppError {
    AppError::validation(message)
}
fn io_error(_: impl std::fmt::Debug) -> AppError {
    failure("The local backup file could not be read or written. Check the folder and available disk space.")
}
fn read_archive(path: &Path) -> Result<Vec<u8>, AppError> {
    let file = File::open(path).map_err(io_error)?;
    if !file.metadata().map_err(io_error)?.is_file() {
        return Err(failure("Choose a regular local backup file."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_ARCHIVE + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err(failure("This backup exceeds the 256 MiB archive limit."));
    }
    Ok(bytes)
}
pub struct SelectedBackup {
    pub original: PathBuf,
    pub file_hash: String,
    pub manifest: Manifest,
    pub counts: Counts,
    pub staging: tempfile::TempDir,
}
impl SelectedBackup {
    pub fn snapshot(&self) -> PathBuf {
        self.staging.path().join("bookbuddy.sqlite3")
    }
}

pub async fn export(
    pool: &SqlitePool,
    directory: &Path,
    destination: &Path,
) -> Result<Manifest, AppError> {
    if destination.exists() {
        return Err(failure(
            "Choose a new backup filename. Existing files are preserved.",
        ));
    }
    fs::create_dir_all(directory).map_err(io_error)?;
    let staging = tempfile::tempdir_in(directory).map_err(io_error)?;
    let snapshot = snapshots::snapshot(pool, staging.path(), "manual").await?;
    let connection = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&snapshot)
            .read_only(true),
    )
    .await?;
    let version: i64 = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
        .fetch_one(&connection)
        .await?;
    let covers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM book WHERE cover_path IS NOT NULL")
        .fetch_one(&connection)
        .await?;
    connection.close().await;
    if covers != 0 {
        return Err(failure("This version cannot export cover assets. Keep the full data folder and use a compatible version."));
    }
    let bytes = fs::read(snapshot).map_err(io_error)?;
    if bytes.len() as u64 > MAX_DATABASE {
        return Err(failure(
            "The library exceeds this backup format's 512 MiB database limit.",
        ));
    }
    let manifest = Manifest {
        format: "BookBuddy".into(),
        version: 1,
        schema_version: version,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(io_error)?,
        database_sha256: hash(&bytes),
        database_bytes: bytes.len() as u64,
        cover_assets: Vec::new(),
    };
    let parent = destination
        .parent()
        .ok_or_else(|| failure("Choose a local backup folder."))?;
    let mut output = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    {
        let mut archive = ZipWriter::new(output.as_file_mut());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        archive
            .start_file("manifest.json", options)
            .map_err(io_error)?;
        archive
            .write_all(&serde_json::to_vec_pretty(&manifest).map_err(io_error)?)
            .map_err(io_error)?;
        archive
            .start_file("database.sqlite3", options)
            .map_err(io_error)?;
        archive.write_all(&bytes).map_err(io_error)?;
        archive.finish().map_err(io_error)?;
    }
    if output.as_file().metadata().map_err(io_error)?.len() > MAX_ARCHIVE {
        return Err(failure(
            "The compressed backup exceeds 256 MiB. No export was installed.",
        ));
    }
    output.as_file().sync_all().map_err(io_error)?;
    output.persist_noclobber(destination).map_err(io_error)?;
    Ok(manifest)
}
pub async fn inspect(path: &Path, staging_directory: &Path) -> Result<SelectedBackup, AppError> {
    let bytes = read_archive(path)?;
    let file_hash = hash(&bytes);
    let mut archive = ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| failure("This file is not a supported BookBuddy backup archive."))?;
    if archive.len() != 2 {
        return Err(failure(
            "The backup archive must contain exactly a manifest and database snapshot.",
        ));
    }
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(io_error)?;
        if !matches!(file.name(), "manifest.json" | "database.sqlite3")
            || file.is_dir()
            || file.is_symlink()
        {
            return Err(failure(
                "The backup contains unsupported or unsafe archive entries.",
            ));
        }
    }
    let manifest: Manifest = {
        let file = archive.by_name("manifest.json").map_err(io_error)?;
        if file.size() > 32 * 1024 {
            return Err(failure("The backup manifest is too large."));
        }
        let mut bytes = Vec::new();
        file.take(32 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        serde_json::from_slice(&bytes).map_err(|_| failure("The backup manifest is invalid."))?
    };
    if manifest.format != "BookBuddy"
        || manifest.version != 1
        || !manifest.cover_assets.is_empty()
        || manifest.database_bytes > MAX_DATABASE
        || manifest.database_bytes == 0
    {
        return Err(failure(
            "This backup format or its assets are not supported by this version.",
        ));
    }
    fs::create_dir_all(staging_directory).map_err(io_error)?;
    let staging = tempfile::tempdir_in(staging_directory).map_err(io_error)?;
    let snapshot = staging.path().join("bookbuddy.sqlite3");
    {
        let file = archive.by_name("database.sqlite3").map_err(io_error)?;
        if file.size() != manifest.database_bytes {
            return Err(failure("The database size does not match the manifest."));
        }
        let mut bytes = Vec::new();
        file.take(MAX_DATABASE + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() as u64 != manifest.database_bytes || hash(&bytes) != manifest.database_sha256
        {
            return Err(failure(
                "The backup database checksum does not match. The library was not changed.",
            ));
        }
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&snapshot)
            .map_err(io_error)?;
        output.write_all(&bytes).map_err(io_error)?;
        output.sync_all().map_err(io_error)?;
    }
    let checked = repository::validate_snapshot(&snapshot, manifest.schema_version).await?;
    checked.close().await;
    // Upgrade only the disposable, schema-verified staging copy. The archive and
    // household database stay unchanged throughout inspect/preview.
    let migrated = persistence::open(staging.path())
        .await
        .map_err(|_| failure("The backup could not be upgraded safely in its staging folder."))?;
    let counts = repository::counts(&migrated).await?;
    migrated.close().await;
    Ok(SelectedBackup {
        original: path.to_path_buf(),
        file_hash,
        manifest,
        counts,
        staging,
    })
}
pub async fn preview(
    pool: &SqlitePool,
    selected: &SelectedBackup,
    token: &str,
) -> Result<RestorePreview, AppError> {
    Ok(RestorePreview {
        token: token.into(),
        source_name: selected
            .original
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        manifest: selected.manifest.clone(),
        backup_counts: selected.counts.clone(),
        current_counts: repository::counts(pool).await?,
        review_hash: repository::state_hash(pool).await?,
    })
}
pub async fn restore(
    pool: &SqlitePool,
    directory: &Path,
    selected: &SelectedBackup,
    review_hash: &str,
) -> Result<String, AppError> {
    if hash(&read_archive(&selected.original)?) != selected.file_hash {
        return Err(failure(
            "The selected backup changed after preview. Choose it again.",
        ));
    }
    if repository::state_hash(pool).await? != review_hash {
        return Err(failure(
            "The library changed after restore preview. Refresh the preview before replacing it.",
        ));
    }
    let version = sqlx::migrate!("./migrations")
        .iter()
        .map(|migration| migration.version)
        .max()
        .unwrap_or_default();
    let checked = repository::validate_snapshot(&selected.snapshot(), version).await?;
    checked.close().await;
    let backup = directory
        .join("backups")
        .join(format!("pre-restore-{}.bookbuddy", uuid::Uuid::new_v4()));
    fs::create_dir_all(backup.parent().unwrap()).map_err(io_error)?;
    export(pool, directory, &backup).await?;
    repository::replace_from_snapshot(pool, &selected.snapshot(), None).await?;
    pool.close().await;
    Ok(backup.file_name().unwrap().to_string_lossy().into_owned())
}
fn automatic_files(directory: &Path) -> Result<Vec<(std::time::SystemTime, PathBuf)>, AppError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(id) = name
            .strip_prefix("automatic-")
            .and_then(|name| name.strip_suffix(".bookbuddy"))
        else {
            continue;
        };
        if uuid::Uuid::parse_str(id).is_err() {
            continue;
        }
        let metadata = entry.metadata().map_err(io_error)?;
        if metadata.is_file() && !entry.file_type().map_err(io_error)?.is_symlink() {
            files.push((metadata.modified().map_err(io_error)?, entry.path()));
        }
    }
    files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    Ok(files)
}
pub async fn automatic(pool: &SqlitePool, directory: &Path, force: bool) -> Result<(), AppError> {
    let settings = repository::settings(pool).await?;
    if !settings.enabled {
        return Ok(());
    }
    let backups = directory.join("backups");
    let files = automatic_files(&backups)?;
    let due = force
        || files.first().is_none_or(|(modified, _)| {
            modified
                .elapsed()
                .is_ok_and(|age| age.as_secs() >= 24 * 60 * 60)
        });
    if due {
        fs::create_dir_all(&backups).map_err(io_error)?;
        export(
            pool,
            directory,
            &backups.join(format!("automatic-{}.bookbuddy", uuid::Uuid::new_v4())),
        )
        .await?;
    }
    // Recovery/manual files and unknown names are never subject to rotation.
    for (_, path) in automatic_files(&backups)?
        .into_iter()
        .skip(settings.keep as usize)
    {
        fs::remove_file(path).map_err(io_error)?;
    }
    Ok(())
}
pub async fn status(
    pool: &SqlitePool,
    directory: &Path,
    automatic_problem: bool,
) -> Result<BackupStatus, AppError> {
    Ok(BackupStatus {
        settings: repository::settings(pool).await?,
        automatic_count: automatic_files(&directory.join("backups"))?.len(),
        automatic_problem,
        backup_directory: directory.join("backups").to_string_lossy().into_owned(),
    })
}
