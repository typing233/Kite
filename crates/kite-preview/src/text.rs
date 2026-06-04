use crate::traits::{PreviewContent, PreviewError, PreviewProvider, StyledLine, StyledSpan};
use ratatui::style::{Color, Style};
use std::path::Path;
use syntect::highlighting::{ThemeSet, Theme};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;

pub struct TextPreviewProvider {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl TextPreviewProvider {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Self { syntax_set, theme }
    }

    pub fn with_theme(theme_name: &str) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .unwrap_or_else(|| theme_set.themes["base16-ocean.dark"].clone());
        Self { syntax_set, theme }
    }
}

impl PreviewProvider for TextPreviewProvider {
    fn supported_extensions(&self) -> &[&str] {
        &[
            "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "c", "cpp", "h", "hpp",
            "java", "go", "rb", "sh", "bash", "zsh", "fish", "toml", "yaml", "yml",
            "json", "xml", "html", "css", "scss", "lua", "vim", "sql", "zig", "swift",
            "kt", "scala", "makefile", "dockerfile", "gitignore", "env", "ini", "cfg",
            "conf", "log",
        ]
    }

    fn can_preview(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        self.supported_extensions().contains(&ext.to_lowercase().as_str())
            || matches!(
                name.to_lowercase().as_str(),
                "makefile" | "dockerfile" | ".gitignore" | ".env" | "cargo.lock"
            )
    }

    fn preview(&self, path: &Path, _width: u16, height: u16) -> Result<PreviewContent, PreviewError> {
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > 10 * 1024 * 1024 {
            return Err(PreviewError::TooLarge { size: metadata.len() });
        }

        let content = std::fs::read_to_string(path)
            .map_err(|_| PreviewError::Other("Failed to read as UTF-8".to_string()))?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();
        let max_lines = height as usize;

        for (i, line) in LinesWithEndings::from(&content).enumerate() {
            if i >= max_lines {
                break;
            }

            let highlighted = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let spans: Vec<StyledSpan> = highlighted
                .into_iter()
                .map(|(style, text)| {
                    let fg = Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    StyledSpan {
                        text: text.to_string(),
                        style: Style::default().fg(fg),
                    }
                })
                .collect();

            lines.push(StyledLine { spans });
        }

        Ok(PreviewContent::StyledText(lines))
    }

    fn priority(&self) -> u8 {
        60
    }
}
