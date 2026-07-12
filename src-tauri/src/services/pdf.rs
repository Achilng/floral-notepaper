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
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            page_size: "A4".into(),
            font_family: "HarmonyOS Sans".into(),
            font_size: 14,
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
    fn source(&self, _id: FileId) -> FileResult<Source> {
        Ok(self.source.clone())
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::Other(None))
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 7, 12)
    }
}

fn markdown_to_typst(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut in_code_block = false;

    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_code_block {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if line.starts_with("###### ") {
            out.push_str("====== ");
            out.push_str(&line[7..]);
        } else if line.starts_with("##### ") {
            out.push_str("===== ");
            out.push_str(&line[6..]);
        } else if line.starts_with("#### ") {
            out.push_str("==== ");
            out.push_str(&line[5..]);
        } else if line.starts_with("### ") {
            out.push_str("=== ");
            out.push_str(&line[4..]);
        } else if line.starts_with("## ") {
            out.push_str("== ");
            out.push_str(&line[3..]);
        } else if line.starts_with("# ") {
            out.push_str("= ");
            out.push_str(&line[2..]);
        } else {
            let converted = convert_inline(line);
            out.push_str(&converted);
        }
        out.push('\n');
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
    let converted = markdown_to_typst(content);

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
        let id = face.id.clone();
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
