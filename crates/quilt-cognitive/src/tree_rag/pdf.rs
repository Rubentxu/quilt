//! PDF renderer — converts Markdown to PDF using genpdf + pulldown-cmark

use crate::tree_rag::types::TreeRagConfig;
use genpdf::{elements, style, Alignment, Document};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Attempt to load the default font family from disk or system fonts.
fn load_default_font() -> Option<genpdf::fonts::FontFamily<genpdf::fonts::FontData>> {
    let paths = [
        "assets/fonts/Roboto-Regular.ttf",
        "crates/quilt-cognitive/assets/fonts/Roboto-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    let bold_paths = [
        "assets/fonts/Roboto-Bold.ttf",
        "crates/quilt-cognitive/assets/fonts/Roboto-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    ];

    let regular_data = paths.iter().find_map(|p| std::fs::read(p).ok())?;
    let bold_data = bold_paths
        .iter()
        .find_map(|p| std::fs::read(p).ok())
        .unwrap_or_else(|| regular_data.clone());

    let regular_font = genpdf::fonts::FontData::new(regular_data.clone(), None).ok()?;
    let bold_font = genpdf::fonts::FontData::new(bold_data, None).ok()?;
    let italic_font = genpdf::fonts::FontData::new(regular_data, None).ok()?;
    let bold_italic_font = italic_font.clone();

    Some(genpdf::fonts::FontFamily {
        regular: regular_font,
        bold: bold_font,
        italic: italic_font,
        bold_italic: bold_italic_font,
    })
}

/// Renders a Markdown string to a PDF byte vector.
pub fn render_markdown_to_pdf(
    markdown: &str,
    _config: &TreeRagConfig,
) -> Result<Vec<u8>, String> {
    let font_family = load_default_font()
        .ok_or_else(|| "PDF fonts not found.".to_string())?;

    let mut doc = Document::new(font_family);
    doc.set_title("Quilt Report");

    let parser = Parser::new_ext(markdown, Options::all());
    let mut heading_level: Option<u8> = None;
    let mut current_text = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => {
                flush_text(&mut current_text, heading_level, &mut doc);
                if let Tag::Heading { level, .. } = tag {
                    heading_level = Some(level as u8);
                }
            }
            Event::End(tag) => {
                if matches!(tag, TagEnd::Heading(_)) {
                    flush_text(&mut current_text, heading_level, &mut doc);
                    heading_level = None;
                }
            }
            Event::Text(text) | Event::Code(text) => {
                current_text.push_str(&text);
            }
            Event::SoftBreak => {
                current_text.push(' ');
            }
            Event::HardBreak => {
                flush_text(&mut current_text, heading_level, &mut doc);
            }
            _ => {}
        }
    }
    flush_text(&mut current_text, heading_level, &mut doc);

    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("PDF render error: {}", e))?;
    Ok(buffer)
}

fn flush_text(text: &mut String, heading_level: Option<u8>, doc: &mut Document) {
    if text.trim().is_empty() {
        text.clear();
        return;
    }

    let paragraph = elements::Paragraph::new(text.as_str());
    let para = if let Some(_level) = heading_level {
        doc.push(elements::Break::new(0));
        paragraph.aligned(Alignment::Left) // genpdf 0.2: aligned(), not align()
    } else {
        paragraph.aligned(Alignment::Left)
    };

    doc.push(para);
    text.clear();
}
