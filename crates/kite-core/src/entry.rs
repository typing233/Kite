use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub permissions: Permissions,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Pipe,
    Socket,
    BlockDevice,
    CharDevice,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    pub user: PermTriple,
    pub group: PermTriple,
    pub other: PermTriple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermTriple {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permissions {
    pub fn from_mode(mode: u32) -> Self {
        Self {
            user: PermTriple {
                read: mode & 0o400 != 0,
                write: mode & 0o200 != 0,
                execute: mode & 0o100 != 0,
            },
            group: PermTriple {
                read: mode & 0o040 != 0,
                write: mode & 0o020 != 0,
                execute: mode & 0o010 != 0,
            },
            other: PermTriple {
                read: mode & 0o004 != 0,
                write: mode & 0o002 != 0,
                execute: mode & 0o001 != 0,
            },
        }
    }

    pub fn display(&self) -> String {
        format!(
            "{}{}{}{}{}{}{}{}{}",
            if self.user.read { 'r' } else { '-' },
            if self.user.write { 'w' } else { '-' },
            if self.user.execute { 'x' } else { '-' },
            if self.group.read { 'r' } else { '-' },
            if self.group.write { 'w' } else { '-' },
            if self.group.execute { 'x' } else { '-' },
            if self.other.read { 'r' } else { '-' },
            if self.other.write { 'w' } else { '-' },
            if self.other.execute { 'x' } else { '-' },
        )
    }
}

impl FileEntry {
    pub fn icon(&self) -> &'static str {
        match self.kind {
            EntryKind::Directory => "\u{f115}",
            EntryKind::Symlink => "\u{f0c1}",
            EntryKind::File => icon_for_extension(
                self.path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or(""),
            ),
            _ => "\u{f016}",
        }
    }

    pub fn is_text_file(&self) -> bool {
        if self.kind != EntryKind::File {
            return false;
        }
        let ext = self.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        matches!(
            ext,
            "txt" | "md" | "rs" | "py" | "js" | "ts" | "jsx" | "tsx"
                | "c" | "cpp" | "h" | "hpp" | "java" | "go" | "rb"
                | "sh" | "bash" | "zsh" | "fish" | "toml" | "yaml"
                | "yml" | "json" | "xml" | "html" | "css" | "scss"
                | "lua" | "vim" | "el" | "lisp" | "sql" | "zig"
                | "swift" | "kt" | "scala" | "r" | "R" | "makefile"
                | "dockerfile" | "gitignore" | "env" | "ini" | "cfg"
                | "conf" | "log"
        )
    }

    pub fn is_image_file(&self) -> bool {
        if self.kind != EntryKind::File {
            return false;
        }
        let ext = self
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "ico" | "svg"
        )
    }
}

fn icon_for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "rs" => "\u{e7a8}",
        "toml" => "\u{e615}",
        "md" => "\u{e73e}",
        "py" => "\u{e73c}",
        "js" | "jsx" => "\u{e74e}",
        "ts" | "tsx" => "\u{e628}",
        "go" => "\u{e626}",
        "c" | "h" => "\u{e61e}",
        "cpp" | "hpp" | "cc" => "\u{e61d}",
        "java" => "\u{e738}",
        "rb" => "\u{e739}",
        "sh" | "bash" | "zsh" => "\u{e795}",
        "json" => "\u{e60b}",
        "yaml" | "yml" => "\u{e6a8}",
        "html" => "\u{e736}",
        "css" | "scss" => "\u{e749}",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => "\u{f1c5}",
        "pdf" => "\u{f1c1}",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "\u{f1c6}",
        "lock" => "\u{f023}",
        "lua" => "\u{e620}",
        "vim" => "\u{e62b}",
        "git" | "gitignore" => "\u{f1d3}",
        "docker" | "dockerfile" => "\u{f308}",
        _ => "\u{f016}",
    }
}
