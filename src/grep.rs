use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

/// Plafond de résultats pour rester réactif sur de gros fichiers.
const MAX_RESULTS: usize = 200;

/// Une ligne correspondant au motif dans un des fichiers recherchés.
pub struct GrepMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub line: String,
}

pub struct GrepOutcome {
    pub matches: Vec<GrepMatch>,
    pub truncated: bool,
}

/// Cherche `pattern` (regex) ligne par ligne dans chaque fichier de `paths`.
/// Les dossiers de la sélection sont ignorés (recherche non récursive), tout
/// comme les fichiers illisibles en texte (binaires, encodage invalide...).
pub fn search_in_files(paths: &[PathBuf], pattern: &str) -> Result<GrepOutcome, regex::Error> {
    let regex = Regex::new(pattern)?;
    let mut matches = Vec::new();
    let mut truncated = false;

    'files: for path in paths {
        if !path.is_file() {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else { continue };
        for (idx, line) in content.lines().enumerate() {
            if matches.len() >= MAX_RESULTS {
                truncated = true;
                break 'files;
            }
            if regex.is_match(line) {
                matches.push(GrepMatch { path: path.clone(), line_number: idx + 1, line: line.to_owned() });
            }
        }
    }

    Ok(GrepOutcome { matches, truncated })
}

/// `true` si `path` désigne un fichier régulier (pas un dossier) — utilisé
/// pour vérifier qu'au moins un élément "cherchable" est sélectionné avant
/// d'ouvrir le pop-up de saisie du motif.
pub fn is_searchable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("directory-octopus-test-grep-{name}-{}", std::process::id()));
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
    fn finds_matching_lines_with_their_line_number() {
        let tmp = TempDir::new("basic");
        let file = tmp.0.join("a.txt");
        fs::write(&file, "hello world\nfoo bar\nhello again\n").unwrap();

        let outcome = search_in_files(&[file.clone()], "hello").unwrap();
        assert_eq!(outcome.matches.len(), 2);
        assert_eq!(outcome.matches[0].line_number, 1);
        assert_eq!(outcome.matches[0].line, "hello world");
        assert_eq!(outcome.matches[1].line_number, 3);
        assert!(!outcome.truncated);
    }

    #[test]
    fn supports_regular_expressions() {
        let tmp = TempDir::new("regex");
        let file = tmp.0.join("a.txt");
        fs::write(&file, "foo123\nbar\nfoo456\n").unwrap();

        let outcome = search_in_files(&[file], r"foo\d+").unwrap();
        assert_eq!(outcome.matches.len(), 2);
    }

    #[test]
    fn searches_across_multiple_files() {
        let tmp = TempDir::new("multi");
        let a = tmp.0.join("a.txt");
        let b = tmp.0.join("b.txt");
        fs::write(&a, "match here\n").unwrap();
        fs::write(&b, "match there\n").unwrap();

        let outcome = search_in_files(&[a, b], "match").unwrap();
        assert_eq!(outcome.matches.len(), 2);
    }

    #[test]
    fn skips_directories_in_the_selection() {
        let tmp = TempDir::new("skip-dir");
        let dir = tmp.0.join("subdir");
        fs::create_dir_all(&dir).unwrap();
        let file = tmp.0.join("a.txt");
        fs::write(&file, "match\n").unwrap();

        let outcome = search_in_files(&[dir, file], "match").unwrap();
        assert_eq!(outcome.matches.len(), 1);
    }

    #[test]
    fn invalid_regex_returns_an_error() {
        let tmp = TempDir::new("invalid-regex");
        let file = tmp.0.join("a.txt");
        fs::write(&file, "hello\n").unwrap();

        assert!(search_in_files(&[file], "(unterminated").is_err());
    }
}
