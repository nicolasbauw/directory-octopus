use std::path::{Path, PathBuf};

use chrono::Local;
use sysinfo::{Disk, Disks};

/// Le disque (point de montage Unix, lettre de lecteur/volume Windows/macOS —
/// `sysinfo` abstrait tout ça) contenant `path` : celui dont le point de
/// montage préfixe `path` le plus précisément (ex: une clé USB montée sous
/// /run/media/... plutôt que la racine /).
fn disk_containing<'a>(disks: &'a Disks, path: &Path) -> Option<&'a Disk> {
    disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
}

/// Espace total et libre (en octets) du système de fichiers contenant
/// `path`, via `sysinfo` (appels système natifs, portable). `None` si aucun
/// point de montage connu ne correspond (ne devrait pas arriver en pratique).
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    let disks = Disks::new_with_refreshed_list();
    disk_containing(&disks, path).map(|disk| (disk.total_space(), disk.available_space()))
}

/// La racine du support de stockage contenant `path` (le point de montage
/// lui-même), pour le bouton "Root" : ramène par exemple de
/// /run/media/nicolasb/SamData/OneDrive à /run/media/nicolasb/SamData,
/// plutôt qu'à la racine générale du système de fichiers "/".
pub fn device_root(path: &Path) -> Option<PathBuf> {
    let disks = Disks::new_with_refreshed_list();
    disk_containing(&disks, path).map(|disk| disk.mount_point().to_path_buf())
}

/// Formate une taille en octets de façon compacte (ex: "500B", "12K",
/// "8.7M", "1.3G"), comme sur l'écran de référence Directory Opus.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0}{}", UNITS[unit])
    } else {
        format!("{value:.1}{}", UNITS[unit])
    }
}

/// Date et heure courantes, dans un format calqué sur celui de l'utilitaire
/// Unix `date` (ex: "Wed Sep 16 21:44:53 +02:00 2026"). Le fuseau est affiché
/// en décalage numérique plutôt qu'en abréviation ("CEST") : `chrono` reste
/// ainsi portable sans dépendre d'une base de données de fuseaux du système.
pub fn current_date_time() -> String {
    Local::now().format("%a %b %e %H:%M:%S %:z %Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_picks_the_right_unit_and_precision() {
        assert_eq!(human_size(0), "0B");
        assert_eq!(human_size(500), "500B");
        assert_eq!(human_size(2048), "2.0K");
        assert_eq!(human_size(1_500_000), "1.4M");
        assert_eq!(human_size(10_000_000_000), "9.3G");
    }
}
