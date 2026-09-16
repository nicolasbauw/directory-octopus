use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Un raccourci personnalisable de la colonne de gauche, tel que persisté sur
/// disque (miroir de `button_bar::DriveSlot`, sans dépendre de son module).
#[derive(Serialize, Deserialize, Clone)]
pub struct DriveSlotConfig {
    pub label: String,
    pub path: Option<PathBuf>,
}

/// Paramétrage utilisateur persisté entre deux lancements.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub drive_slots: Vec<DriveSlotConfig>,
}

fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("directory-octopus");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_owned());
    PathBuf::from(home).join(".config").join("directory-octopus")
}

fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

impl AppConfig {
    /// Retourne `None` si aucun fichier de configuration n'existe encore ou
    /// s'il est illisible/invalide — l'appelant doit alors utiliser ses
    /// valeurs par défaut plutôt que planter.
    pub fn load() -> Option<Self> {
        let content = std::fs::read_to_string(config_file()).ok()?;
        toml::from_str(&content).ok()
    }

    /// Écrit la configuration sur disque. Échoue silencieusement (ex :
    /// permissions, disque plein) plutôt que d'interrompre l'utilisateur pour
    /// un simple problème de sauvegarde de préférences.
    pub fn save(&self) {
        let Ok(content) = toml::to_string_pretty(self) else { return };
        if std::fs::create_dir_all(config_dir()).is_ok() {
            let _ = std::fs::write(config_file(), content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_roundtrips_drive_slots() {
        let tmp = std::env::temp_dir().join(format!("directory-octopus-test-{}", std::process::id()));
        // SAFETY: ce test est le seul de ce process à lire/écrire cette
        // variable, donc pas de course avec d'autres tests exécutés en
        // parallèle dans le même binaire.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &tmp) };

        let config = AppConfig {
            drive_slots: vec![
                DriveSlotConfig { label: "HOME:".into(), path: Some(PathBuf::from("/home/user")) },
                DriveSlotConfig { label: "BOOKMARKS:".into(), path: None },
            ],
        };
        config.save();

        let loaded = AppConfig::load().expect("le fichier vient d'être écrit");
        assert_eq!(loaded.drive_slots.len(), 2);
        assert_eq!(loaded.drive_slots[0].label, "HOME:");
        assert_eq!(loaded.drive_slots[0].path, Some(PathBuf::from("/home/user")));
        assert_eq!(loaded.drive_slots[1].path, None);

        let _ = std::fs::remove_dir_all(&tmp);
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
    }
}
