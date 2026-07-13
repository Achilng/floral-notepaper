use std::collections::HashMap;
use std::path::Path;
use typst::diag::{FileError, FileResult, Warned};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::World;
use typst::{Library, LibraryExt};

#[derive(Debug, Clone)]
pub struct PdfExportOptions {
    pub page_size: String,
    pub font_family: String,
    pub font_size: u32,
    pub admonition_labels: HashMap<String, String>,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        let mut admonition_labels = HashMap::new();
        admonition_labels.insert("note".into(), "Note".into());
        admonition_labels.insert("tip".into(), "Tip".into());
        admonition_labels.insert("important".into(), "Important".into());
        admonition_labels.insert("warning".into(), "Warning".into());
        admonition_labels.insert("caution".into(), "Caution".into());
        Self {
            page_size: "a4".into(),
            font_family: "HarmonyOS Sans".into(),
            font_size: 14,
            admonition_labels,
        }
    }
}

struct ExportWorld {
    source: Source,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    library: LazyHash<Library>,
}

impl World for ExportWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, _id: FileId) -> FileResult<Source> {
        Ok(self.source.clone())
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::Other(None))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 7, 12)
    }
}

fn admonition_icon(type_name: &str) -> String {
    match type_name.to_uppercase().as_str() {
        "NOTE" => r#"#box(
  fill: rgb(9, 105, 218),
  width: 1.2em,
  height: 1.2em,
  radius: 50%,
  inset: 0pt,
  align(center + horizon, text(fill: white, size: 0.8em, weight: "bold", "i"))
)"#
        .to_string(),
        "TIP" => r#"#box(
  fill: rgb(26, 127, 55),
  width: 1.2em,
  height: 1.2em,
  radius: 50%,
  inset: 0pt,
  align(center + horizon, text(fill: white, size: 0.8em, "★"))
)"#
        .to_string(),
        "WARNING" => r#"#box(
  fill: rgb(154, 103, 0),
  width: 1.2em,
  height: 1.2em,
  radius: 50%,
  inset: 0pt,
  align(center + horizon, text(fill: white, size: 0.8em, "!"))
)"#
        .to_string(),
        "CAUTION" => r#"#box(
  fill: rgb(207, 34, 46),
  width: 1.2em,
  height: 1.2em,
  radius: 50%,
  inset: 0pt,
  align(center + horizon, text(fill: white, size: 0.8em, weight: "bold", "!"))
)"#
        .to_string(),
        "IMPORTANT" => r#"#box(
  fill: rgb(130, 80, 223),
  width: 1.2em,
  height: 1.2em,
  radius: 50%,
  inset: 0pt,
  align(center + horizon, text(fill: white, size: 0.8em, "◆"))
)"#
        .to_string(),
        _ => "".to_string(),
    }
}

fn markdown_to_typst(md: &str, labels: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(md.len());
    let mut in_code_block = false;

    let lines: Vec<&str> = md.lines().collect();
    let len = lines.len();
    let mut i = 0;

    while i < len {
        let line = lines[i];

        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }
        if in_code_block {
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }

        // > [!TYPE] [optional content]
        if let Some(rest) = line.strip_prefix("> [!") {
            if let Some(end) = rest.find(']') {
                let type_name = &rest[..end].to_lowercase();
                let inline_rest = rest[end + 1..].trim();
                let kind = type_name.as_str();
                let (accent, bg) = match kind {
                    "note" => ("rgb(9, 105, 218)", "rgb(212, 231, 250)"),
                    "tip" => ("rgb(26, 127, 55)", "rgb(210, 240, 218)"),
                    "warning" => ("rgb(154, 103, 0)", "rgb(250, 236, 210)"),
                    "caution" => ("rgb(207, 34, 46)", "rgb(253, 220, 222)"),
                    "important" => ("rgb(130, 80, 223)", "rgb(230, 220, 250)"),
                    _ => ("rgb(9, 105, 218)", "rgb(212, 231, 250)"),
                };
                let label = labels.get(kind).map(|s| s.as_str()).unwrap_or(kind);
                let icon = admonition_icon(kind);
                out.push_str(&format!(
                    "#block(fill: {bg}, stroke: (left: 4pt + {accent}), \
             inset: (x: 1em, y: 0.75em), radius: 4pt, width: 100%, breakable: true)["
                ));
                if !icon.is_empty() {
                    out.push_str(icon.as_str());
                    out.push('\n');
                }
                out.push_str(&format!("*{}:*", label));
                out.push('\n');
                if !inline_rest.is_empty() {
                    out.push_str(&convert_inline(inline_rest));
                    out.push('\n');
                }

                i += 1;
                while i < len {
                    let aline = lines[i];
                    if let Some(content) = aline.strip_prefix("> ") {
                        if content.trim().is_empty() {
                            out.push_str("\n\n");
                        } else {
                            out.push(' ');
                            out.push_str(&convert_inline(content));
                        }
                        i += 1;
                    } else {
                        break;
                    }
                }
                out.push_str("]\n\n");
                continue;
            }
        }

        if let Some(rest) = line.strip_prefix("###### ") {
            out.push_str("====== ");
            out.push_str(rest);
        } else if let Some(rest) = line.strip_prefix("##### ") {
            out.push_str("===== ");
            out.push_str(rest);
        } else if let Some(rest) = line.strip_prefix("#### ") {
            out.push_str("==== ");
            out.push_str(rest);
        } else if let Some(rest) = line.strip_prefix("### ") {
            out.push_str("=== ");
            out.push_str(rest);
        } else if let Some(rest) = line.strip_prefix("## ") {
            out.push_str("== ");
            out.push_str(rest);
        } else if let Some(rest) = line.strip_prefix("# ") {
            out.push_str("= ");
            out.push_str(rest);
        } else {
            let converted = convert_inline(line);
            out.push_str(&converted);
        }
        out.push('\n');
        i += 1;
    }

    out
}

fn convert_inline(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    while i < n {
        // *** — markdown bold+italic → typst _*text*_
        if i + 2 < n && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*' {
            result.push_str("_*");
            i += 3;
            let start = i;
            while i + 2 < n && !(chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*') {
                i += 1;
            }
            if i + 2 < n {
                for &c in &chars[start..i] {
                    result.push(c);
                }
                result.push_str("*_");
                i += 3;
            } else {
                for &c in &chars[start..] {
                    result.push(c);
                }
                break;
            }
            continue;
        }
        // ** — markdown bold → typst _text_
        if i + 1 < n && chars[i] == '*' && chars[i + 1] == '*' {
            result.push('_');
            i += 2;
            let start = i;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '*') {
                i += 1;
            }
            if i + 1 < n {
                for &c in &chars[start..i] {
                    result.push(c);
                }
                result.push('_');
                i += 2;
            } else {
                for &c in &chars[start..] {
                    result.push(c);
                }
                break;
            }
            continue;
        }
        // `code` — same in typst
        if chars[i] == '`' {
            let start = i;
            while i < n && chars[i] != '`' {
                i += 1;
            }
            if i < n {
                for &c in &chars[start..=i] {
                    result.push(c);
                }
                i += 1;
            } else {
                for &c in &chars[start..] {
                    result.push(c);
                }
                break;
            }
            continue;
        }
        // 这里跳过特殊字符，避免转义错误
        if chars[i] == '\\' {
            result.push_str("\\\\");
            i += 1;
            continue;
        }
        if chars[i] == '#' {
            result.push_str("\\#");
            i += 1;
            continue;
        }
        if chars[i] == '$' {
            result.push_str("\\$");
            i += 1;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn build_typst_source(content: &str, options: &PdfExportOptions) -> String {
    let size = options.page_size.to_lowercase();
    let font = &options.font_family;
    let size_pt = options.font_size;
    let converted = markdown_to_typst(content, &options.admonition_labels);

    format!(
        "#set page(paper: \"{size}\", margin: 2.5cm)\n\
         #set text(font: (\"{font}\", \"Noto Sans SC\", \"PingFang SC\", \"Microsoft YaHei\"), size: {size_pt}pt)\n\
         \n\
         {converted}\n"
    )
}

pub fn export_markdown_to_pdf(
    content: &str,
    path: &Path,
    options: &PdfExportOptions,
) -> Result<(), String> {
    let typst_source = build_typst_source(content, options);
    let world = create_world(&typst_source)?;
    let Warned {
        output,
        warnings: _,
    } = typst::compile(&world);
    let document = output.map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        format!("Typst compilation failed: {}", msgs.join("; "))
    })?;
    let pdf_bytes =
        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
            let msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
            format!("PDF export failed: {}", msgs.join("; "))
        })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create directory: {e}"))?;
    }
    std::fs::write(path, &pdf_bytes).map_err(|e| format!("write PDF: {e}"))?;
    Ok(())
}

fn create_world(source: &str) -> Result<ExportWorld, String> {
    let vpath = VirtualPath::new("main.typ").map_err(|e| format!("invalid virtual path: {e}"))?;
    let rooted = RootedPath::new(VirtualRoot::Project, vpath);
    let id = FileId::new(rooted);
    let source = Source::new(id, source.into());

    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();

    let mut book = FontBook::new();
    let mut fonts = Vec::new();

    for face in fontdb.faces() {
        let id = face.id;
        let index = face.index;
        if let Some(Some(font)) = fontdb.with_face_data(id, move |data: &[u8], _: u32| {
            Font::new(Bytes::new(data.to_vec()), index)
        }) {
            book.push(font.info().clone());
            fonts.push(font);
        }
    }

    Ok(ExportWorld {
        source,
        book: LazyHash::new(book),
        fonts,
        library: LazyHash::new(Library::builder().build()),
    })
}
