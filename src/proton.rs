//! Detección automática de prefixes de Proton/Wine para sincronización
//! de saves entre Windows y SteamOS.
//!
//! Proton almacena los saves de Windows dentro de un "prefix" en:
//!   ~/.steam/steam/steamapps/compatdata/<AppID>/pfx/
//!
//! Este módulo localiza ese prefix dado un Steam App ID,
//! buscando en todas las instalaciones de Steam conocidas en el sistema.

use crate::path::StrictPath;

/// Rutas estándar donde puede estar Steam en Linux/SteamOS.
/// Las devuelve en orden de prioridad (la más común primero).
pub fn default_steam_roots() -> Vec<StrictPath> {
    let mut roots = vec![];

    let Some(home) = dirs::home_dir() else {
        log::warn!("[proton] No se pudo determinar el directorio home");
        return roots;
    };
    let home = home.to_string_lossy().to_string();

    // Steam nativo - ruta simbólica estándar
    roots.push(StrictPath::new(format!("{}/.steam/steam", home)));

    // Steam nativo - ruta real en algunas distros
    roots.push(StrictPath::new(format!("{}/.local/share/Steam", home)));

    // Steam Flatpak
    roots.push(StrictPath::new(format!(
        "{}/.var/app/com.valvesoftware.Steam/.steam/steam",
        home
    )));

    // Steam Snap (Ubuntu)
    roots.push(StrictPath::new(format!(
        "{}/snap/steam/common/.steam/steam",
        home
    )));

    roots
}

/// Dado un Steam App ID y una lista de raíces de Steam,
/// devuelve la ruta al prefix de Proton (`pfx/`) si existe.
///
/// Ejemplo de ruta resultante:
///   `/home/deck/.steam/steam/steamapps/compatdata/367520/pfx`
pub fn find_proton_prefix(app_id: u32, steam_roots: &[StrictPath]) -> Option<StrictPath> {
    for root in steam_roots {
        let pfx = root.joined(&format!("steamapps/compatdata/{}/pfx", app_id));
        log::trace!("[proton] comprobando prefix: {:?}", &pfx);
        if pfx.is_dir() {
            log::debug!("[proton] prefix encontrado para AppID {}: {:?}", app_id, &pfx);
            return Some(pfx);
        }
    }
    log::debug!("[proton] no se encontró prefix para AppID {}", app_id);
    None
}

/// Devuelve true si el sistema actual parece ser SteamOS o Linux con Steam.
/// Útil para decidir si ofrecer la autodetección de Proton en la UI.
pub fn is_steam_linux_environment() -> bool {
    if cfg!(target_os = "linux") {
        // STEAMOS_VERSION existe en SteamOS, SteamDeck=1 en Steam Deck
        let is_steamos = std::env::var("STEAMOS_VERSION").is_ok()
            || std::env::var("SteamDeck").as_deref() == Ok("1");

        // Si no es SteamOS oficial, comprobamos si al menos existe algún root de Steam
        if is_steamos {
            return true;
        }
        return default_steam_roots().iter().any(|r| r.is_dir());
    }
    false
}

/// Resultado de inspeccionar un prefix de Proton.
#[derive(Debug, Clone)]
pub struct ProtonPrefixInfo {
    /// Ruta al directorio `pfx/`
    pub pfx: StrictPath,
    /// Ruta a `pfx/drive_c/` — equivalente a `C:\` en Windows
    pub drive_c: StrictPath,
    /// Ruta al perfil del usuario de Steam dentro del prefix
    /// (`pfx/drive_c/users/steamuser/`)
    pub steamuser: StrictPath,
    /// True si el prefix parece válido (contiene drive_c y el usuario)
    pub is_valid: bool,
}

impl ProtonPrefixInfo {
    /// Construye la info a partir de la ruta al `pfx/`.
    pub fn from_pfx(pfx: StrictPath) -> Self {
        let drive_c = pfx.joined("drive_c");
        let steamuser = drive_c.joined("users/steamuser");
        let is_valid = drive_c.is_dir() && steamuser.is_dir();

        if !is_valid {
            log::warn!(
                "[proton] prefix encontrado pero parece incompleto: {:?}",
                &pfx
            );
        }

        Self {
            pfx,
            drive_c,
            steamuser,
            is_valid,
        }
    }

    /// Versión conveniente: busca el prefix y construye la info en un paso.
    pub fn find(app_id: u32, steam_roots: &[StrictPath]) -> Option<Self> {
        let pfx = find_proton_prefix(app_id, steam_roots)?;
        Some(Self::from_pfx(pfx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_steam_roots_are_not_empty() {
        // En cualquier sistema con home dir debería devolver al menos una ruta
        let roots = default_steam_roots();
        assert!(!roots.is_empty());
    }

    #[test]
    fn find_proton_prefix_returns_none_for_nonexistent_root() {
        let fake_roots = vec![StrictPath::new("/nonexistent/path/that/does/not/exist")];
        assert!(find_proton_prefix(370, &fake_roots).is_none());
    }

    #[test]
    fn prefix_info_invalid_for_nonexistent_path() {
        let info = ProtonPrefixInfo::from_pfx(StrictPath::new("/nonexistent/pfx"));
        assert!(!info.is_valid);
    }
}
