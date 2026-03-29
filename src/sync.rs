//! EmuSync-style cross-OS save synchronization engine.
//!
//! Flujo:
//!   scan (cross_os) → zip neutro → rclone upload
//!   rclone download → unzip → rutas locales del dispositivo

use std::{
    io::{Read, Write},
    path::PathBuf,
    time::SystemTime,
};

use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::scan::cross_os::SaveLocation;

// ---------------------------------------------------------------------------
// Errores
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum SyncError {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    NoSaveLocations,
    TimestampError,
}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<zip::result::ZipError> for SyncError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Zip(e)
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Zip(e) => write!(f, "Zip error: {e}"),
            Self::NoSaveLocations => write!(f, "No save locations found for game"),
            Self::TimestampError => write!(f, "Could not determine file timestamp"),
        }
    }
}

// ---------------------------------------------------------------------------
// Escaneo de directorio
// Port de LocalDataAccessor.ScanDirectory() de EmuSync
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct DirectoryScanResult {
    pub directory_is_set: bool,
    pub directory_exists: bool,
    pub latest_file_mtime: Option<SystemTime>,
    pub storage_bytes: u64,
    pub file_count: u64,
}

impl DirectoryScanResult {
    /// El timestamp más reciente entre archivos — equivale a LatestWriteTimeUtc de EmuSync
    pub fn latest_write_time(&self) -> Option<SystemTime> {
        self.latest_file_mtime
    }
}

/// Escanea un directorio recursivamente y devuelve stats.
/// Port de LocalDataAccessor.ScanDirectory() + SearchDirectory() de EmuSync.
pub fn scan_directory(path: Option<&std::path::Path>) -> DirectoryScanResult {
    let mut result = DirectoryScanResult::default();

    let Some(path) = path else {
        result.directory_is_set = false;
        return result;
    };

    result.directory_is_set = true;
    result.directory_exists = path.is_dir();

    if !result.directory_exists {
        return result;
    }

    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .flatten()
    {
        let Ok(meta) = entry.metadata() else { continue };

        if meta.is_file() {
            result.file_count += 1;
            result.storage_bytes += meta.len();

            if let Ok(mtime) = meta.modified() {
                match result.latest_file_mtime {
                    None => result.latest_file_mtime = Some(mtime),
                    Some(current) if mtime > current => result.latest_file_mtime = Some(mtime),
                    _ => {}
                }
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Zip neutro
// Port de ZipHelper.CreateZipFromFolder() de EmuSync.
// Estructura interna: archivos directamente sin prefijo, rutas relativas al save_dir.
// ---------------------------------------------------------------------------

/// Crea un zip neutro con todos los saves de la ubicación dada.
/// Devuelve los bytes del zip.
/// Port de ZipHelper.CreateZipFromFolder().
pub fn pack_saves(save_dir: &std::path::Path) -> Result<Vec<u8>, SyncError> {
    let buf = Vec::new();
    let cursor = std::io::Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);

    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for entry in walkdir::WalkDir::new(save_dir)
        .follow_links(true)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();

        // Ruta relativa dentro del zip — sin prefijo, igual que EmuSync
        let relative = file_path
            .strip_prefix(save_dir)
            .unwrap_or(file_path);
        let zip_path = relative.to_string_lossy().replace('\\', "/");

        zip.start_file(&zip_path, options)?;
        let mut f = std::fs::File::open(file_path)?;
        let mut contents = Vec::new();
        f.read_to_end(&mut contents)?;
        zip.write_all(&contents)?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

/// Extrae un zip neutro en el directorio de saves del dispositivo actual.
/// Port de ZipHelper.ExtractToDirectory() de EmuSync.
pub fn unpack_saves(zip_bytes: &[u8], target_dir: &std::path::Path) -> Result<(), SyncError> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if name.ends_with('/') {
            continue; // directorio
        }

        let target_path = target_dir.join(&name);

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = std::fs::File::create(&target_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        out.write_all(&contents)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Lógica newest-wins
// Port de DetermineSyncType() de EmuSync
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncDecision {
    /// Los saves locales son más nuevos → subir
    Upload,
    /// Los saves en la nube son más nuevos → bajar  
    Download,
    /// Mismo timestamp → nada que hacer
    AlreadySynced,
    /// No hay directorio local ni registro en nube
    Unknown,
    /// El directorio local no está configurado
    UnsetDirectory,
}

/// Metadatos guardados en la nube junto al zip.
/// Equivale a game.LatestWriteTimeUtc de EmuSync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncMetadata {
    /// Timestamp del save más reciente (Unix segundos UTC)
    pub latest_mtime_secs: u64,
    /// Dispositivo que hizo el último upload
    pub device: String,
    /// Cuándo se hizo el último sync
    pub last_sync_utc: u64,
}

impl SyncMetadata {
    pub fn new(mtime: SystemTime, device: impl Into<String>) -> Result<Self, SyncError> {
        let latest_mtime_secs = mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| SyncError::TimestampError)?
            .as_secs();

        let last_sync_utc = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| SyncError::TimestampError)?
            .as_secs();

        Ok(Self {
            latest_mtime_secs,
            device: device.into(),
            last_sync_utc,
        })
    }

    pub fn as_system_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(self.latest_mtime_secs)
    }
}

/// Decide qué dirección sincronizar.
/// Port exacto de DetermineSyncType() de EmuSync.
pub fn decide_sync_direction(
    scan: &DirectoryScanResult,
    remote_metadata: Option<&SyncMetadata>,
) -> SyncDecision {
    // Nunca se ha sincronizado
    let Some(remote) = remote_metadata else {
        if scan.directory_exists {
            return SyncDecision::Upload;
        }
        return SyncDecision::Unknown;
    };

    if !scan.directory_is_set {
        return SyncDecision::UnsetDirectory;
    }

    // El directorio local no existe pero hay registro en nube → descargar
    if !scan.directory_exists {
        return SyncDecision::Download;
    }

    let local_mtime = scan.latest_write_time().unwrap_or(SystemTime::UNIX_EPOCH);
    let remote_mtime = remote.as_system_time();

    // Sin margen — igual que EmuSync
    if local_mtime > remote_mtime {
        SyncDecision::Upload
    } else if local_mtime < remote_mtime {
        SyncDecision::Download
    } else {
        SyncDecision::AlreadySynced
    }
}

// ---------------------------------------------------------------------------
// Paths de trabajo local para los zips temporales
// Port de GetTempZipPath() de EmuSync
// ---------------------------------------------------------------------------

pub fn temp_zip_path(game_name: &str) -> PathBuf {
    let mut dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    dir.push("ludusavi-emusync");
    dir.push("temp");
    std::fs::create_dir_all(&dir).ok();
    dir.push(format!("{}.zip", sanitize_game_name(game_name)));
    dir
}

pub fn metadata_path(game_name: &str) -> PathBuf {
    let mut dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    dir.push("ludusavi-emusync");
    std::fs::create_dir_all(&dir).ok();
    dir.push(format!("{}.meta.json", sanitize_game_name(game_name)));
    dir
}

fn sanitize_game_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_when_no_remote() {
        let scan = DirectoryScanResult {
            directory_is_set: true,
            directory_exists: true,
            latest_file_mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100)),
            ..Default::default()
        };
        assert_eq!(SyncDecision::Upload, decide_sync_direction(&scan, None));
    }

    #[test]
    fn unknown_when_no_remote_and_no_local() {
        let scan = DirectoryScanResult {
            directory_is_set: true,
            directory_exists: false,
            ..Default::default()
        };
        assert_eq!(SyncDecision::Unknown, decide_sync_direction(&scan, None));
    }

    #[test]
    fn upload_when_local_newer() {
        let scan = DirectoryScanResult {
            directory_is_set: true,
            directory_exists: true,
            latest_file_mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000)),
            ..Default::default()
        };
        let remote = SyncMetadata {
            latest_mtime_secs: 100,
            device: "other".into(),
            last_sync_utc: 0,
        };
        assert_eq!(SyncDecision::Upload, decide_sync_direction(&scan, Some(&remote)));
    }

    #[test]
    fn download_when_remote_newer() {
        let scan = DirectoryScanResult {
            directory_is_set: true,
            directory_exists: true,
            latest_file_mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100)),
            ..Default::default()
        };
        let remote = SyncMetadata {
            latest_mtime_secs: 1000,
            device: "other".into(),
            last_sync_utc: 0,
        };
        assert_eq!(SyncDecision::Download, decide_sync_direction(&scan, Some(&remote)));
    }

    #[test]
    fn already_synced_when_equal() {
        let scan = DirectoryScanResult {
            directory_is_set: true,
            directory_exists: true,
            latest_file_mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000)),
            ..Default::default()
        };
        let remote = SyncMetadata {
            latest_mtime_secs: 1000,
            device: "other".into(),
            last_sync_utc: 0,
        };
        assert_eq!(SyncDecision::AlreadySynced, decide_sync_direction(&scan, Some(&remote)));
    }

    #[test]
    fn download_when_no_local_dir_but_remote_exists() {
        let scan = DirectoryScanResult {
            directory_is_set: true,
            directory_exists: false,
            ..Default::default()
        };
        let remote = SyncMetadata {
            latest_mtime_secs: 1000,
            device: "other".into(),
            last_sync_utc: 0,
        };
        assert_eq!(SyncDecision::Download, decide_sync_direction(&scan, Some(&remote)));
    }
}
