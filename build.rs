use std::path::Path;
use std::process::Command;

/// Hash court du commit sur lequel ce binaire a été buildé, affiché dans le
/// pop-up "About". Deux sources, dans l'ordre :
/// - `GIT_COMMIT` : substitué par `git archive` (donc par les tarballs de
///   release GitHub, cf. `.gitattributes`) — fonctionne même sans `.git/`.
/// - à défaut (checkout normal, `PKGBUILD-git`), `git rev-parse` en direct.
fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    println!("cargo:rerun-if-changed=GIT_COMMIT");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let from_export_subst = std::fs::read_to_string(Path::new(manifest_dir).join("GIT_COMMIT"))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty() && !s.starts_with("$Format"));

    let hash = from_export_subst
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=8", "HEAD"])
                .current_dir(manifest_dir)
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned());

    println!("cargo:rustc-env=GIT_HASH={hash}");
}
