//! `install` — place the running carpenter binary into a bin dir (default from
//! config, else `~/.local/bin`) so it is callable globally.

use crate::core::error::CarpenterError;
use crate::core::store::{self, Paths};
use crate::models::Data;

/// `install [--bin-dir <p>]`.
pub fn install(paths: &Paths, bin_dir: Option<&str>) -> Result<Data, CarpenterError> {
    let target_dir = paths.resolve_bin_dir(bin_dir);
    let src = std::env::current_exe()
        .map_err(|e| CarpenterError::StoreError(format!("current_exe failed: {e}")))?;
    std::fs::create_dir_all(&target_dir).map_err(store::io_to_store)?;
    let dest = target_dir.join("carpenter");
    let already_present = dest.exists();
    std::fs::copy(&src, &dest).map_err(store::io_to_store)?;
    Ok(Data::Install {
        installed: true,
        already_present,
        bin: dest.display().to_string(),
        on_path: store::is_on_path(&target_dir),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;

    #[test]
    fn install_copies_binary_into_bin_dir() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        let Data::Install {
            installed,
            already_present,
            bin,
            on_path: _,
        } = install(&paths, Some(bin_dir.to_str().unwrap())).expect("install")
        else {
            panic!("Install");
        };
        assert!(installed);
        assert!(!already_present, "fresh target did not exist");
        assert!(bin.ends_with("bin/carpenter"), "{bin}");
        assert!(
            std::path::Path::new(&bin).exists(),
            "binary should exist at {bin}"
        );
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn install_reports_already_present_on_reinstall() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        let Data::Install {
            already_present, ..
        } = install(&paths, Some(bin_dir.to_str().unwrap())).expect("first install")
        else {
            panic!("Install");
        };
        assert!(!already_present);
        let Data::Install {
            already_present, ..
        } = install(&paths, Some(bin_dir.to_str().unwrap())).expect("reinstall")
        else {
            panic!("Install");
        };
        assert!(already_present, "re-install over an existing binary");
        let _ = std::fs::remove_dir_all(&paths.root);
    }
}
