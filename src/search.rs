use std::fs;
use std::path::{Path, PathBuf};

use glob::{MatchOptions, Pattern};

/// Plafond de résultats pour rester réactif sur de grosses arborescences.
const MAX_RESULTS: usize = 200;

/// Résultat d'une recherche : les correspondances trouvées, et si le plafond
/// `MAX_RESULTS` a été atteint (il pourrait donc y en avoir davantage sur le
/// disque que ce qui est listé ici).
pub struct FindOutcome {
    pub matches: Vec<PathBuf>,
    pub truncated: bool,
}

/// Recherche récursive, depuis `root`, des fichiers/dossiers dont le nom
/// correspond au motif (`*` et `?` supportés, insensible à la casse).
/// Parcourt le système de fichiers en direct (pas d'index à rafraîchir, donc
/// jamais périmé) et fonctionne identiquement sur toutes les plateformes.
pub fn find_matches(root: &Path, pattern: &str) -> FindOutcome {
    let Ok(pattern) = Pattern::new(pattern) else { return FindOutcome { matches: Vec::new(), truncated: false } };
    let options = MatchOptions { case_sensitive: false, require_literal_separator: false, require_literal_leading_dot: false };

    let mut results = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        if results.len() >= MAX_RESULTS {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            if results.len() >= MAX_RESULTS {
                break;
            }
            let name = entry.file_name();
            if pattern.matches_with(&name.to_string_lossy(), options) {
                results.push(entry.path());
            }
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                pending.push(entry.path());
            }
        }
    }
    let truncated = results.len() >= MAX_RESULTS;
    results.sort();
    FindOutcome { matches: results, truncated }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("directory-octopus-test-search-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finds_files_matching_a_star_pattern_recursively() {
        let tmp = TempDir::new("star");
        fs::create_dir_all(tmp.0.join("sub")).unwrap();
        fs::write(tmp.0.join("report.txt"), b"").unwrap();
        fs::write(tmp.0.join("sub").join("notes.txt"), b"").unwrap();
        fs::write(tmp.0.join("image.png"), b"").unwrap();

        let mut outcome = find_matches(&tmp.0, "*.txt");
        outcome.matches.sort();
        assert_eq!(outcome.matches, vec![tmp.0.join("report.txt"), tmp.0.join("sub").join("notes.txt")]);
        assert!(!outcome.truncated);
    }

    #[test]
    fn question_mark_matches_exactly_one_character() {
        let tmp = TempDir::new("question");
        fs::write(tmp.0.join("a.txt"), b"").unwrap();
        fs::write(tmp.0.join("ab.txt"), b"").unwrap();

        let outcome = find_matches(&tmp.0, "?.txt");
        assert_eq!(outcome.matches, vec![tmp.0.join("a.txt")]);
    }

    #[test]
    fn matching_is_case_insensitive() {
        let tmp = TempDir::new("case");
        fs::write(tmp.0.join("Report.TXT"), b"").unwrap();

        let outcome = find_matches(&tmp.0, "report.txt");
        assert_eq!(outcome.matches, vec![tmp.0.join("Report.TXT")]);
    }

    #[test]
    fn reports_truncated_when_the_result_cap_is_reached() {
        let tmp = TempDir::new("truncated");
        for i in 0..250 {
            fs::write(tmp.0.join(format!("f{i}.txt")), b"").unwrap();
        }

        let outcome = find_matches(&tmp.0, "*.txt");
        assert_eq!(outcome.matches.len(), 200);
        assert!(outcome.truncated);
    }

    #[test]
    fn invalid_pattern_returns_no_results_instead_of_panicking() {
        let tmp = TempDir::new("invalid");
        fs::write(tmp.0.join("a.txt"), b"").unwrap();

        assert_eq!(find_matches(&tmp.0, "[unterminated").matches, Vec::<PathBuf>::new());
    }
}
