use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn dest_path(src: &Path, dst_dir: &Path) -> io::Result<PathBuf> {
    let name = src
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "source sans nom de fichier"))?;
    Ok(dst_dir.join(name))
}

/// Copie un fichier, ou récursivement un dossier, dans `dst_dir` (résultat :
/// `dst_dir/<nom de src>`).
pub fn copy_into(src: &Path, dst_dir: &Path) -> io::Result<()> {
    let dst = dest_path(src, dst_dir)?;
    if src.is_dir() { copy_dir_recursive(src, &dst) } else { fs::copy(src, &dst).map(|_| ()) }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dst_child = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_child)?;
        } else {
            fs::copy(entry.path(), &dst_child)?;
        }
    }
    Ok(())
}

/// Déplace `src` dans `dst_dir`. Tente un renommage atomique (rapide, même
/// système de fichiers) et se rabat sur copie+suppression sinon (ex :
/// périphériques de stockage différents).
pub fn move_into(src: &Path, dst_dir: &Path) -> io::Result<()> {
    let dst = dest_path(src, dst_dir)?;
    if fs::rename(src, &dst).is_ok() {
        return Ok(());
    }
    copy_into(src, dst_dir)?;
    delete(src)
}

/// Supprime un fichier, ou récursivement un dossier.
pub fn delete(path: &Path) -> io::Result<()> {
    if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) }
}

/// `true` si `path` est un dossier contenant au moins une entrée (sert à
/// décider si une confirmation de suppression est nécessaire).
pub fn is_non_empty_dir(path: &Path) -> bool {
    path.is_dir() && fs::read_dir(path).map(|mut it| it.next().is_some()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dossier temporaire unique, nettoyé à la fin du test.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("directory-octopus-test-{name}-{}", std::process::id()));
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
    fn copy_into_copies_a_file() {
        let tmp = TempDir::new("copy-file");
        let src_dir = tmp.0.join("src");
        let dst_dir = tmp.0.join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        let file = src_dir.join("a.txt");
        fs::write(&file, b"hello").unwrap();

        copy_into(&file, &dst_dir).unwrap();

        assert_eq!(fs::read(dst_dir.join("a.txt")).unwrap(), b"hello");
        assert!(file.exists(), "la source ne doit pas être supprimée par une copie");
    }

    #[test]
    fn copy_into_copies_a_directory_recursively() {
        let tmp = TempDir::new("copy-dir");
        let src_dir = tmp.0.join("src").join("folder");
        let dst_dir = tmp.0.join("dst");
        fs::create_dir_all(src_dir.join("nested")).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        fs::write(src_dir.join("nested").join("b.txt"), b"world").unwrap();

        copy_into(&src_dir, &dst_dir).unwrap();

        assert_eq!(fs::read(dst_dir.join("folder").join("nested").join("b.txt")).unwrap(), b"world");
    }

    #[test]
    fn move_into_relocates_and_removes_the_source() {
        let tmp = TempDir::new("move");
        let src_dir = tmp.0.join("src");
        let dst_dir = tmp.0.join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        let file = src_dir.join("a.txt");
        fs::write(&file, b"hello").unwrap();

        move_into(&file, &dst_dir).unwrap();

        assert!(!file.exists());
        assert_eq!(fs::read(dst_dir.join("a.txt")).unwrap(), b"hello");
    }

    #[test]
    fn delete_removes_files_and_non_empty_directories() {
        let tmp = TempDir::new("delete");
        let file = tmp.0.join("a.txt");
        fs::write(&file, b"hi").unwrap();
        delete(&file).unwrap();
        assert!(!file.exists());

        let dir = tmp.0.join("folder");
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("nested").join("b.txt"), b"hi").unwrap();
        delete(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn is_non_empty_dir_distinguishes_empty_from_populated() {
        let tmp = TempDir::new("nonempty");
        let empty = tmp.0.join("empty");
        let populated = tmp.0.join("populated");
        fs::create_dir_all(&empty).unwrap();
        fs::create_dir_all(&populated).unwrap();
        fs::write(populated.join("f.txt"), b"hi").unwrap();

        assert!(!is_non_empty_dir(&empty));
        assert!(is_non_empty_dir(&populated));
        assert!(!is_non_empty_dir(&tmp.0.join("f.txt"))); // n'existe pas -> pas un dossier
    }
}
