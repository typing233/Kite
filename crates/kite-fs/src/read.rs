use kite_core::entry::{EntryKind, FileEntry, Permissions};
use std::path::Path;
use tokio::fs;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub async fn read_directory(path: &Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let mut dir = fs::read_dir(path).await?;

    while let Some(entry) = dir.next_entry().await? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let file_path = entry.path();
        let is_hidden = file_name.starts_with('.');

        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        let (kind, is_symlink, symlink_target) = if metadata.is_symlink() {
            let target = fs::read_link(&file_path).await.ok();
            let resolved_kind = if metadata.is_dir() {
                EntryKind::Directory
            } else if metadata.is_file() {
                EntryKind::File
            } else {
                EntryKind::Symlink
            };
            (resolved_kind, true, target)
        } else {
            let kind = determine_kind(&metadata);
            (kind, false, None)
        };

        #[cfg(unix)]
        let permissions = Permissions::from_mode(metadata.mode());
        #[cfg(not(unix))]
        let permissions = Permissions::from_mode(0o644);

        let size = metadata.len();
        let modified = metadata.modified().ok();

        entries.push(FileEntry {
            name: file_name,
            path: file_path,
            kind,
            size,
            modified,
            permissions,
            is_hidden,
            is_symlink,
            symlink_target,
        });
    }

    Ok(entries)
}

fn determine_kind(metadata: &std::fs::Metadata) -> EntryKind {
    if metadata.is_dir() {
        EntryKind::Directory
    } else if metadata.is_file() {
        EntryKind::File
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            let ft = metadata.file_type();
            if ft.is_fifo() {
                EntryKind::Pipe
            } else if ft.is_socket() {
                EntryKind::Socket
            } else if ft.is_block_device() {
                EntryKind::BlockDevice
            } else if ft.is_char_device() {
                EntryKind::CharDevice
            } else {
                EntryKind::Unknown
            }
        }
        #[cfg(not(unix))]
        {
            EntryKind::Unknown
        }
    }
}
