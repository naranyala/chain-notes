//! PDF export for the active note.
//!
//! The exporter deliberately has no egui or file-picker dependency. The UI
//! chooses a path; this module turns one note into a small, readable PDF.

use std::fs;
use std::io;
use std::path::Path;

use crate::note::Note;

pub fn export_note(path: &Path, note: &Note) -> io::Result<()> {
    let lines = content_lines(note);
    let per_page = 42;
    let pages = lines.chunks(per_page).collect::<Vec<_>>();
    let mut objects = Vec::new();

    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let page_tree_object = 2;
    objects.push(Vec::new());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());

    let mut page_refs = Vec::new();
    for page_lines in pages {
        let content_object = objects.len() + 2;
        let page_object = objects.len() + 1;
        page_refs.push(page_object);
        objects.push(format!(
            "<< /Type /Page /Parent {page_tree_object} 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_object} 0 R >>"
        ).into_bytes());
        let stream = page_stream(page_lines);
        objects.push(
            format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                stream.len(),
                stream
            )
            .into_bytes(),
        );
    }
    let kids = page_refs
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects[page_tree_object - 1] = format!(
        "<< /Type /Pages /Kids [{kids}] /Count {} >>",
        page_refs.len()
    )
    .into_bytes();

    let mut pdf = b"%PDF-1.4\n%\xFF\xFF\xFF\xFF\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    fs::write(path, pdf)
}

fn content_lines(note: &Note) -> Vec<String> {
    let created = crate::calendar::from_unix_seconds(note.created_at);
    let updated = crate::calendar::from_unix_seconds(note.updated_at);
    let mut lines = vec![
        sanitize(&note.title),
        format!(
            "Created: {:04}-{:02}-{:02} UTC | Updated: {:04}-{:02}-{:02} UTC",
            created.year, created.month, created.day, updated.year, updated.month, updated.day
        ),
        String::new(),
    ];
    if note.body.is_empty() {
        lines.push("(No body text)".to_owned());
    } else {
        for source in note.body.lines() {
            let text = sanitize(source);
            if text.is_empty() {
                lines.push(String::new());
            } else {
                lines.extend(wrap(&text, 92));
            }
        }
    }
    lines
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            result.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        result.push(line);
    }
    result
}

fn sanitize(text: &str) -> String {
    text.chars()
        .map(|ch| if ch.is_ascii() { ch } else { '?' })
        .collect()
}

fn page_stream(lines: &[String]) -> String {
    let mut stream = String::from("BT\n/F1 18 Tf\n54 738 Td\n");
    for (index, line) in lines.iter().enumerate() {
        if index == 1 {
            stream.push_str("/F1 9 Tf\n");
        } else if index == 2 {
            stream.push_str("/F1 11 Tf\n");
        }
        stream.push_str(&format!("({}) Tj\n0 -16 Td\n", escape(line)));
    }
    stream.push_str("ET");
    stream
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn export_writes_a_pdf_with_note_text_and_metadata() {
        let path =
            std::env::temp_dir().join(format!("chainnotes-export-{}.pdf", std::process::id()));
        let mut note = Note::new(1, "Export title");
        note.body = "A short body (with punctuation).".to_owned();
        export_note(&path, &note).expect("export");
        let bytes = fs::read(&path).expect("read export");
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("Export title"));
        assert!(text.contains("Created:"));
        assert!(text.contains("A short body \\(with punctuation\\)."));
        if std::env::var_os("CHAINNOTES_KEEP_PDF").is_none() {
            let _ = fs::remove_file(path);
        }
    }
}
