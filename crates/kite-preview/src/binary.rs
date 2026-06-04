use crate::traits::{PreviewContent, PreviewError, PreviewProvider, StyledLine, StyledSpan};
use ratatui::style::{Color, Style};
use std::io::Read;
use std::path::Path;

pub struct BinaryPreviewProvider;

impl PreviewProvider for BinaryPreviewProvider {
    fn supported_extensions(&self) -> &[&str] {
        &[]
    }

    fn can_preview(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn preview(&self, path: &Path, _width: u16, height: u16) -> Result<PreviewContent, PreviewError> {
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();

        let mut file = std::fs::File::open(path)?;
        let bytes_to_read = std::cmp::min(size, (height as u64) * 16);
        let mut buffer = vec![0u8; bytes_to_read as usize];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        let mut lines = Vec::new();

        // Header with file size
        let size_str = format_size(size);
        lines.push(StyledLine {
            spans: vec![StyledSpan {
                text: format!("  Binary file ({})", size_str),
                style: Style::default().fg(Color::Yellow),
            }],
        });
        lines.push(StyledLine {
            spans: vec![StyledSpan {
                text: String::new(),
                style: Style::default(),
            }],
        });

        // Hex dump
        let bytes_per_line = 16;
        for (i, chunk) in buffer.chunks(bytes_per_line).enumerate() {
            if lines.len() >= height as usize {
                break;
            }

            let offset = i * bytes_per_line;
            let hex_part: String = chunk
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");

            let ascii_part: String = chunk
                .iter()
                .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
                .collect();

            let padding = " ".repeat((bytes_per_line - chunk.len()) * 3);

            lines.push(StyledLine {
                spans: vec![
                    StyledSpan {
                        text: format!("  {:08x}  ", offset),
                        style: Style::default().fg(Color::DarkGray),
                    },
                    StyledSpan {
                        text: hex_part,
                        style: Style::default().fg(Color::Cyan),
                    },
                    StyledSpan {
                        text: padding,
                        style: Style::default(),
                    },
                    StyledSpan {
                        text: format!("  {}", ascii_part),
                        style: Style::default().fg(Color::Green),
                    },
                ],
            });
        }

        Ok(PreviewContent::StyledText(lines))
    }

    fn priority(&self) -> u8 {
        10
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
