use kite_core::entry::{EntryKind, FileEntry};
use kite_core::event::SortField;

pub fn sort_entries(entries: &mut [FileEntry], field: SortField, ascending: bool, dirs_first: bool) {
    entries.sort_by(|a, b| {
        if dirs_first {
            let a_is_dir = a.kind == EntryKind::Directory;
            let b_is_dir = b.kind == EntryKind::Directory;
            if a_is_dir != b_is_dir {
                return if a_is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
        }

        let ord = match field {
            SortField::Name => natural_cmp(&a.name.to_lowercase(), &b.name.to_lowercase()),
            SortField::Size => a.size.cmp(&b.size),
            SortField::Modified => a.modified.cmp(&b.modified),
            SortField::Extension => {
                let a_ext = a.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let b_ext = b.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                a_ext.to_lowercase().cmp(&b_ext.to_lowercase())
            }
        };

        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let an = collect_number(&mut ai);
                    let bn = collect_number(&mut bi);
                    match an.cmp(&bn) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    match ac.cmp(&bc) {
                        std::cmp::Ordering::Equal => {
                            ai.next();
                            bi.next();
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

fn collect_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> u64 {
    let mut n: u64 = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            n = n.saturating_mul(10).saturating_add(c as u64 - '0' as u64);
            chars.next();
        } else {
            break;
        }
    }
    n
}
