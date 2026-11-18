//! Extraction des archives téléchargées : .zip (Windows) et .tar.gz (Linux).

use std::path::Path;

/// Extrait `archive` dans `cible`, d'après son extension. Les droits Unix (fichiers exécutables)
/// sont gardés ; un chemin qui sortirait de `cible` est refusé.
pub fn extraire(archive: &Path, cible: &Path) -> anyhow::Result<()> {
    let nom = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let fichier = std::fs::File::open(archive)?;
    if nom.ends_with(".tar.gz") || nom.ends_with(".tgz") {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(fichier));
        tar.set_preserve_permissions(true);
        tar.unpack(cible)?;
    } else {
        zip::ZipArchive::new(fichier)?.extract(cible)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn dossier_test(nom: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("misty-archive-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn tar_gz_et_zip() {
        let d = dossier_test("formats");
        // .tar.gz avec un dossier « Misty-9.9.9 », comme une release Linux
        let tgz = d.join("Misty-9.9.9-linux-x86_64.tar.gz");
        {
            let gz = flate2::write::GzEncoder::new(
                std::fs::File::create(&tgz).unwrap(),
                flate2::Compression::fast(),
            );
            let mut t = tar::Builder::new(gz);
            let mut entete = tar::Header::new_gnu();
            entete.set_size(5);
            entete.set_mode(0o755);
            entete.set_cksum();
            t.append_data(&mut entete, "Misty-9.9.9/Misty", &b"binai"[..])
                .unwrap();
            t.into_inner().unwrap().finish().unwrap();
        }
        extraire(&tgz, &d.join("tgz")).unwrap();
        let exe = d.join("tgz/Misty-9.9.9/Misty");
        assert_eq!(std::fs::read(&exe).unwrap(), b"binai");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&exe).unwrap().permissions().mode() & 0o111,
                0o111
            );
        }

        // .zip, comme une release Windows
        let zip = d.join("Misty-9.9.9.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
            z.start_file(
                "Misty-9.9.9/Misty.exe",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            z.write_all(b"exe").unwrap();
            z.finish().unwrap();
        }
        extraire(&zip, &d.join("zip")).unwrap();
        assert_eq!(
            std::fs::read(d.join("zip/Misty-9.9.9/Misty.exe")).unwrap(),
            b"exe"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
