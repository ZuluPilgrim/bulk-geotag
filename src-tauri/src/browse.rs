//! Inspector: list one folder's subfolders and photos, for browsing inside the app.
//! Read-only. Hidden entries (names starting with ".") are left out, as in the folder scan.
//! Linked (symlinked) folders and photos are listed and marked `linked`: browsing is one
//! folder at a time, so following a link can't loop the way a recursive scan could.

use crate::scan::is_supported;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, PartialEq)]
pub struct Item {
    pub name: String,
    pub path: String,
    /// A symbolic link (to a folder or a photo).
    pub linked: bool,
}

#[derive(Debug, Serialize)]
pub struct FolderListing {
    /// The folder listed (for a file, its folder).
    pub path: String,
    /// The folder above, or None at the top (/ or a drive root).
    pub parent: Option<String>,
    pub folders: Vec<Item>,
    /// Supported photos (jpg, jpeg, png, tif, tiff, webp).
    pub photos: Vec<Item>,
}

pub fn list_folder(path: &str) -> Result<FolderListing, String> {
    let mut dir = Path::new(path);
    if dir.is_file() {
        dir = dir.parent().ok_or_else(|| format!("No folder for {path}"))?;
    }
    if !dir.is_dir() {
        return Err(format!("Not a folder: {path}"));
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("Cannot open {}: {e}", dir.display()))?;
    let mut folders = Vec::new();
    let mut photos = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else { continue };
        // file_type() doesn't follow links; for a link, look at what it points to.
        let linked = kind.is_symlink();
        let (is_dir, is_file) = if linked {
            match fs::metadata(entry.path()) {
                Ok(m) => (m.is_dir(), m.is_file()),
                Err(_) => continue, // broken link
            }
        } else {
            (kind.is_dir(), kind.is_file())
        };
        let item = Item { name, path: entry.path().display().to_string(), linked };
        if is_dir {
            folders.push(item);
        } else if is_file && is_supported(&entry.path()) {
            photos.push(item);
        }
    }
    let key = |i: &Item| i.name.to_lowercase();
    folders.sort_by_key(key);
    photos.sort_by_key(key);
    Ok(FolderListing {
        path: dir.display().to_string(),
        parent: dir.parent().map(|p| p.display().to_string()).filter(|p| !p.is_empty()),
        folders,
        photos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn setup(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bgt-browse-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("Beta")).unwrap();
        fs::create_dir_all(d.join("alpha")).unwrap();
        fs::create_dir_all(d.join(".hidden")).unwrap();
        for f in ["b.JPG", "a.jpg", "notes.txt", ".secret.jpg", "c.tif"] {
            fs::write(d.join(f), b"x").unwrap();
        }
        d
    }

    #[test]
    fn lists_folders_then_photos_sorted_without_hidden() {
        let d = setup("list");
        let l = list_folder(d.to_str().unwrap()).unwrap();
        let names = |v: &Vec<Item>| v.iter().map(|i| i.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&l.folders), ["alpha", "Beta"]);
        assert_eq!(names(&l.photos), ["a.jpg", "b.JPG", "c.tif"]);
        assert_eq!(l.parent.as_deref(), d.parent().and_then(|p| p.to_str()));
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_file_lists_its_folder() {
        let d = setup("file");
        let l = list_folder(d.join("a.jpg").to_str().unwrap()).unwrap();
        assert_eq!(l.path, d.display().to_string());
        assert_eq!(l.photos.len(), 3);
        fs::remove_dir_all(&d).ok();
    }

    #[cfg(unix)]
    #[test]
    fn linked_folders_and_photos_are_listed_and_marked() {
        let d = setup("links");
        let target = d.join("alpha");
        fs::write(target.join("in-alpha.jpg"), b"x").unwrap();
        std::os::unix::fs::symlink(&target, d.join("link-to-alpha")).unwrap();
        std::os::unix::fs::symlink(d.join("a.jpg"), d.join("linked.jpg")).unwrap();
        std::os::unix::fs::symlink(d.join("gone"), d.join("broken")).unwrap();
        let l = list_folder(d.to_str().unwrap()).unwrap();
        let link = l.folders.iter().find(|f| f.name == "link-to-alpha").expect("linked folder listed");
        assert!(link.linked);
        assert!(!l.folders.iter().find(|f| f.name == "alpha").unwrap().linked);
        assert!(l.photos.iter().any(|p| p.name == "linked.jpg" && p.linked));
        assert!(!l.folders.iter().chain(&l.photos).any(|i| i.name == "broken"));
        // Opening the link lists what it points to, and its parent is the folder it's in.
        let inside = list_folder(&link.path).unwrap();
        assert_eq!(inside.photos.len(), 1);
        assert_eq!(inside.parent.as_deref(), d.to_str());
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn missing_folder_is_an_error() {
        assert!(list_folder("/no/such/folder/bgt").is_err());
    }

    #[test]
    fn top_of_the_tree_has_no_parent() {
        let root = if cfg!(windows) { "C:\\" } else { "/" };
        if Path::new(root).is_dir() {
            assert!(list_folder(root).unwrap().parent.is_none());
        }
    }
}
