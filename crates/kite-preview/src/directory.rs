use crate::traits::{PreviewContent, PreviewError, PreviewProvider, StyledLine, StyledSpan};
use ratatui::style::{Color, Style};
use std::path::Path;

pub struct DirectoryPreviewProvider;

impl PreviewProvider for DirectoryPreviewProvider {
    fn supported_extensions(&self) -> &[&str] {
        &[]
    }

    fn can_preview(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn preview(&self, path: &Path, _width: u16, height: u16) -> Result<PreviewContent, PreviewError> {
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .collect();

        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            if a_is_dir != b_is_dir {
                return if a_is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            a.file_name().cmp(&b.file_name())
        });

        let total = entries.len();
        let mut lines = Vec::new();

        // Header
        lines.push(StyledLine {
            spans: vec![StyledSpan {
                text: format!("  {} items", total),
                style: Style::default().fg(Color::Yellow),
            }],
        });
        lines.push(StyledLine {
            spans: vec![StyledSpan {
                text: String::new(),
                style: Style::default(),
            }],
        });

        let max_entries = (height as usize).saturating_sub(3);
        for entry in entries.iter().take(max_entries) {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

            let (icon, color) = if is_dir {
                ("\u{f115} ", Color::Blue)
            } else {
                ("\u{f016} ", Color::White)
            };

            lines.push(StyledLine {
                spans: vec![
                    StyledSpan {
                        text: format!("  {}", icon),
                        style: Style::default().fg(color),
                    },
                    StyledSpan {
                        text: name,
                        style: Style::default().fg(color),
                    },
                ],
            });
        }

        if total > max_entries {
            lines.push(StyledLine {
                spans: vec![StyledSpan {
                    text: format!("  ... and {} more", total - max_entries),
                    style: Style::default().fg(Color::DarkGray),
                }],
            });
        }

        Ok(PreviewContent::StyledText(lines))
    }

    fn priority(&self) -> u8 {
        40
    }
}
