use std::path::Path;

use chrono::Local;
use sysinfo::Disks;

/// Espace total et libre (en octets) du système de fichiers contenant
/// `path`, via `sysinfo` (appels système natifs, portable). `None` si aucun
/// point de montage connu ne correspond (ne devrait pas arriver en pratique).
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    let disks = Disks::new_with_refreshed_list();
    // On choisit le point de montage le plus long qui préfixe `path` : c'est
    // le système de fichiers le plus spécifique (ex: une clé USB montée sous
    // /run/media/... plutôt que la racine /).
    disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| (disk.total_space(), disk.available_space()))
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
