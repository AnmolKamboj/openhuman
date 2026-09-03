//! Native directory chooser for memory-source configuration (#5831).
//!
//! The folder memory-source used to be picked with an
//! `<input type="file" webkitdirectory>` in the renderer. That element hands
//! back `File` objects, and a `File` carries no filesystem location: the
//! `File.path` attribute the old handler read is an Electron extension that
//! no web engine implements. Wry's WKWebView, WebView2 and WebKitGTK do not,
//! and neither does plain Chromium, so the handler always fell through to
//! `webkitRelativePath.split('/')[0]` — the chosen directory's **name**, with
//! its location discarded — and stored that.
//!
//! The resulting source looked configured and could never sync. Because the
//! reader anchors a relative path on the workspace, it failed once per sync
//! cycle, forever, with `folder does not exist: docs (resolved to
//! <workspace>/docs)`. Nothing downstream could repair it: `docs` is not a
//! relative path to the chosen directory, it is a name whose location was
//! thrown away.
//!
//! A host-side dialog closes that gap by construction. It returns an absolute
//! path on every platform, in every renderer, because the OS — not the web
//! engine — owns the selection.
//!
//! ## Trust boundary
//!
//! Deliberately none. Unlike [`crate::artifact_commands`], which re-validates
//! that a renderer-supplied path sits inside the artifacts tree because there
//! the renderer *supplies* the path, this command takes no input at all and
//! returns only what the user chose in an OS-owned dialog. The renderer
//! cannot steer it at a directory, and choosing a folder to index is the
//! user's decision to make anywhere on their own disk.

use std::path::Path;

/// Render a chosen directory as the string the renderer will store, refusing
/// anything that is not absolute.
///
/// Split from the command so it is testable: the dialog itself needs a user
/// and a window server, but this rule is the part worth pinning. The OS
/// choosers all hand back absolute paths, so the rejection is belt-and-braces
/// rather than a branch we expect to take. It exists because a relative value
/// reaching the store is the entire defect this command was written to remove
/// — an error here is recoverable and visible, whereas a stored relative path
/// is neither.
fn absolute_path_string(path: &Path) -> Result<String, String> {
    if !path.is_absolute() {
        return Err(format!(
            "the directory chooser returned a non-absolute path: {}",
            path.display()
        ));
    }
    Ok(path.display().to_string())
}

/// Open the OS-native directory chooser and return the absolute path of the
/// directory the user selected.
///
/// Returns:
/// - `Ok(Some(path))` — the absolute path chosen.
/// - `Ok(None)` — the user dismissed the dialog. Not an error; the caller
///   leaves the field exactly as it was.
/// - `Err(_)` — the dialog could not run, or it somehow yielded a relative
///   path. The caller surfaces this rather than storing anything.
#[tauri::command]
pub async fn pick_directory_via_dialog() -> Result<Option<String>, String> {
    // macOS: NSOpenPanel. Windows: IFileOpenDialog. Linux: the GTK chooser
    // that WebKitGTK already links (see the `gtk3` feature in Cargo.toml).
    // The await resolves when the user picks or cancels.
    let handle = rfd::AsyncFileDialog::new().pick_folder().await;

    let Some(dir) = handle else {
        log::info!("[directory_picker] pick_directory_via_dialog cancelled by user");
        return Ok(None);
    };

    let picked = absolute_path_string(dir.path())?;
    log::info!("[directory_picker] pick_directory_via_dialog chose {picked}");
    Ok(Some(picked))
}

#[cfg(test)]
mod tests {
    use super::absolute_path_string;
    use std::path::Path;

    #[cfg(not(target_os = "windows"))]
    const ABSOLUTE: &str = "/Users/you/notes";
    #[cfg(target_os = "windows")]
    const ABSOLUTE: &str = r"C:\Users\you\notes";

    #[test]
    fn passes_an_absolute_path_through_unchanged() {
        assert_eq!(
            absolute_path_string(Path::new(ABSOLUTE)),
            Ok(ABSOLUTE.to_string())
        );
    }

    #[test]
    fn refuses_a_bare_directory_name() {
        // `docs` is exactly what the old `webkitRelativePath.split('/')[0]`
        // fallback stored, and what made the source unsyncable (#5831).
        let err = absolute_path_string(Path::new("docs")).unwrap_err();
        assert!(err.contains("non-absolute"), "unexpected message: {err}");
        assert!(err.contains("docs"), "message should name the value: {err}");
    }

    #[test]
    fn refuses_a_relative_path_with_separators() {
        assert!(absolute_path_string(Path::new("notes/inner")).is_err());
    }
}
