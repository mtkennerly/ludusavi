//! EmuSync-style cross-OS save synchronization engine.
//!
//! Flujo:
//!   scan (cross_os) → zip neutro → rclone upload
//!   rclone download → unzip → rutas locales del dispositivo

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
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
// Zip neutro
// Empaqueta saves sin rutas absolutas embebidas.
// Estructura interna: saves/nombre_relativo_al_save_dir
// ---------------------------------------------------------------------------

/// Crea un zip neutro con todos los saves de las ubicaciones dadas.
/// Devuelve los bytes del zip y el timestamp más reciente encontrado.
pub fn pack_saves(locations: &[SaveLocation]) -> Result<(Vec<u8>, SystemTime), SyncError> {
    if locations.is_empty() {
        return Err(SyncError::NoSaveLocations);
    }

    let buf = Vec::new();
    let cursor = std::io::Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);

    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let mut latest_mtime = SystemTime::UNIX_EPOCH;

    for location in locations {
        let save_dir = &location.path;

        if !save_dir.is_dir() {
            continue;
        }

        for entry in walkdir::WalkDir::new(save_dir)
            .follow_links(true)
            .into_iter()
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let file_path = entry.path();

            // Ruta relativa dentro del zip: saves/<relativa al save_dir>
            let relative = file_path
                .strip_prefix(save_dir)
                .unwrap_or(file_path);
            let zip_path = format!("saves/{}", relative.to_string_lossy().replace('\\', "/"));

            // Actualizar timestamp más reciente
            if let Ok(meta) = entry.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if mtime > latest_mtime {
                        latest_mtime = mtime;
                    }
                }
            }

            // Añadir al zip
            zip.start_file(&zip_path, options)?;
            let mut f = std::fs::File::open(file_path)?;
            let mut contents = Vec::new();
            f.read_to_end(&mut contents)?;
            zip.write_all(&contents)?;
        }
    }

    let cursor = zip.finish()?;
    Ok((cursor.into_inner(), latest_mtime))
}

/// Extrae un zip neutro en las ubicaciones de save del dispositivo actual.
/// Cada archivo en saves/* se extrae en save_dir/ruta_relativa.
pub fn unpack_saves(zip_bytes: &[u8], locations: &[SaveLocation]) -> Result<(), SyncError> {
    if locations.is_empty() {
        return Err(SyncError::NoSaveLocations);
    }

    // Usamos la primera ubicación como destino principal
    let target_dir = &locations[0].path;

    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        // Solo procesamos entradas bajo saves/
        let Some(relative) = name.strip_prefix("saves/") else {
            continue;
        };

        if relative.is_empty() {
            continue;
        }

        let target_path = target_dir.join(relative);

        // Crear directorios padre si no existen
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
// Timestamps y lógica newest-wins
// ---------------------------------------------------------------------------

/// Metadatos de sync almacenados junto al zip en la nube.
/// Equivale al latestWriteTimeUtc de EmuSync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncMetadata {
    /// Timestamp del save más reciente en el zip (Unix timestamp en segundos)
    pub latest_mtime: u64,
    /// Nombre del dispositivo que hizo el último upload
    pub device: String,
}

impl SyncMetadata {
    pub fn new(mtime: SystemTime, device: impl Into<String>) -> Result<Self, SyncError> {
        let latest_mtime = mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| SyncError::TimestampError)?
            .as_secs();

        Ok(Self {
            latest_mtime,
            device: device.into(),
        })
    }

    pub fn as_system_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(self.latest_mtime)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncDecision {
    /// Los saves locales son más nuevos → subir
    Upload,
    /// Los saves en la nube son más nuevos → bajar
    Download,
    /// Mismo timestamp → nada que hacer
    AlreadySynced,
}

/// Decide qué dirección sincronizar comparando timestamps.
/// Port de la lógica newest-wins de EmuSync.
pub fn decide_sync_direction(
    local_mtime: SystemTime,
    remote_metadata: Option<&SyncMetadata>,
) -> SyncDecision {
    let Some(remote) = remote_metadata else {
        // No hay nada en la nube → subir
        return SyncDecision::Upload;
    };

    let remote_mtime = remote.as_system_time();

    // Margen de 2 segundos para evitar falsos positivos por precisión de FAT32
    let margin = std::time::Duration::from_secs(2);

    match local_mtime.duration_since(remote_mtime) {
        Ok(diff) if diff > margin => SyncDecision::Upload,
        Err(e) if e.duration() > margin => SyncDecision::Download,
        _ => SyncDecision::AlreadySynced,
    }
}

/// Fuerza una dirección concreta ignorando timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcedDirection {
    Upload,
    Download,
}

// ---------------------------------------------------------------------------
// Paths de trabajo local para los zips
// ---------------------------------------------------------------------------

/// Directorio temporal donde se guardan los zips antes de subirlos / tras bajarlos.
pub fn local_zip_dir(game_name: &str) -> PathBuf {
    let mut dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    dir.push("ludusavi-emusync");
    dir.push(sanitize_game_name(game_name));
    dir
}

/// Nombre del zip para un juego.
pub fn zip_filename(game_name: &str) -> String {
    format!("{}.zip", sanitize_game_name(game_name))
}

/// Nombre del fichero de metadatos para un juego.
pub fn metadata_filename(game_name: &str) -> String {
    format!("{}.meta.json", sanitize_game_name(game_name))
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
    fn newest_wins_local_newer() {
        let local = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let remote = SyncMetadata {
            latest_mtime: 100,
            device: "other".to_string(),
        };
        assert_eq!(SyncDecision::Upload, decide_sync_direction(local, Some(&remote)));
    }

    #[test]
    fn newest_wins_remote_newer() {
        let local = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100);
        let remote = SyncMetadata {
            latest_mtime: 1000,
            device: "other".to_string(),
        };
        assert_eq!(SyncDecision::Download, decide_sync_direction(local, Some(&remote)));
    }

    #[test]
    fn newest_wins_no_remote() {
        let local = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100);
        assert_eq!(SyncDecision::Upload, decide_sync_direction(local, None));
    }

    #[test]
    fn newest_wins_same_timestamp() {
        let local = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let remote = SyncMetadata {
            latest_mtime: 1000,
            device: "other".to_string(),
        };
        assert_eq!(SyncDecision::AlreadySynced, decide_sync_direction(local, Some(&remote)));
    }

    #[test]
    fn sanitize_game_name_works() {
        assert_eq!("The_Witcher_3", sanitize_game_name("The Witcher 3"));
        assert_eq!("game-saves_v2", sanitize_game_name("game-saves_v2"));
    }
}
