#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod builder;
mod config;
mod cp437;
mod export;
mod gpg;
mod templates;

use anyhow::{Context, Result};
use encoding_rs::WINDOWS_1252;
use rfd::FileDialog;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

slint::include_modules!();

#[derive(Clone, Copy, PartialEq)]
enum Encoding {
    Cp437,
    Utf8,
    Windows1252,
}

impl Encoding {
    fn name(self) -> &'static str {
        match self {
            Self::Cp437 => "CP437",
            Self::Utf8 => "UTF-8",
            Self::Windows1252 => "Windows-1252",
        }
    }
    fn next(self) -> Self {
        match self {
            Self::Cp437 => Self::Utf8,
            Self::Utf8 => Self::Windows1252,
            Self::Windows1252 => Self::Cp437,
        }
    }
}

#[derive(Clone, Copy)]
enum LineEnding {
    CrLf,
    Lf,
    Cr,
}

impl LineEnding {
    fn name(self) -> &'static str {
        match self {
            Self::CrLf => "CRLF",
            Self::Lf => "LF",
            Self::Cr => "CR",
        }
    }
    fn next(self) -> Self {
        match self {
            Self::CrLf => Self::Lf,
            Self::Lf => Self::Cr,
            Self::Cr => Self::CrLf,
        }
    }
}

struct Document {
    path: Option<PathBuf>,
    dirty: bool,
    encoding: Encoding,
    line_ending: LineEnding,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            path: None,
            dirty: false,
            encoding: Encoding::Cp437,
            line_ending: LineEnding::CrLf,
        }
    }
}

fn main() -> Result<()> {
    let ui = AppWindow::new()?;
    ui.global::<Palette>()
        .set_color_scheme(slint::language::ColorScheme::Dark);
    let state = Rc::new(RefCell::new(Document::default()));
    let gpg_keys = Rc::new(RefCell::new(Vec::<gpg::SigningKey>::new()));
    refresh_chrome(&ui, &state.borrow(), "ready — CP437 mode engaged");
    refresh_stats(&ui, "");
    refresh_gpg_keys(&ui, &gpg_keys);
    ui.set_builder_text(templates::BUILDER_SCENE_RELEASE.into());
    ui.set_builder_preview(
        builder::build_preview(templates::BUILDER_SCENE_RELEASE, "double", 80).into(),
    );
    ui.set_builder_style("double".into());

    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_document_edited(move |text| {
            let Some(ui) = weak.upgrade() else { return };
            state.borrow_mut().dirty = true;
            refresh_stats(&ui, text.as_str());
            refresh_chrome(&ui, &state.borrow(), "modified");
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_new_requested(move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.set_document_text("".into());
            *state.borrow_mut() = Document::default();
            refresh_stats(&ui, "");
            refresh_chrome(&ui, &state.borrow(), "new document");
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_open_requested(move || {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(path) = FileDialog::new()
                .add_filter("NFO and text", &["nfo", "diz", "txt", "asc"])
                .pick_file()
            {
                // Copy the encoding out before entering the match. Keeping
                // `state.borrow()` in the scrutinee holds an immutable Ref
                // through the match arm, where we need `borrow_mut()`.
                let encoding = state.borrow().encoding;
                match open_document(&path, encoding) {
                    Ok((text, ending)) => {
                        ui.set_document_text(text.clone().into());
                        let mut doc = state.borrow_mut();
                        doc.path = Some(path);
                        doc.line_ending = ending;
                        doc.dirty = false;
                        refresh_stats(&ui, &text);
                        refresh_chrome(&ui, &doc, "file opened");
                    }
                    Err(error) => set_error(&ui, error),
                }
            }
        });
    }

    install_save_handler(&ui, state.clone(), false);
    install_save_handler(&ui, state.clone(), true);

    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_encoding_next(move || {
            let Some(ui) = weak.upgrade() else { return };
            let next = state.borrow().encoding.next();
            state.borrow_mut().encoding = next;
            refresh_chrome(&ui, &state.borrow(), "encoding changed (save to apply)");
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_line_ending_next(move || {
            let Some(ui) = weak.upgrade() else { return };
            let next = state.borrow().line_ending.next();
            state.borrow_mut().line_ending = next;
            refresh_chrome(&ui, &state.borrow(), "line endings changed");
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_template_requested(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(template) = templates::TEMPLATES.get(index as usize) {
                ui.set_document_text(template.body.into());
                state.borrow_mut().dirty = true;
                refresh_stats(&ui, template.body);
                refresh_chrome(
                    &ui,
                    &state.borrow(),
                    &format!("loaded {} template", template.name),
                );
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_snippet_requested(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some((_, snippet)) = templates::SNIPPETS.get(index as usize) {
                let mut text = ui.get_document_text().to_string();
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(snippet);
                text.push('\n');
                ui.set_document_text(text.clone().into());
                state.borrow_mut().dirty = true;
                refresh_stats(&ui, &text);
                refresh_chrome(&ui, &state.borrow(), "snippet inserted");
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_format_requested(move |action, width| {
            let Some(ui) = weak.upgrade() else { return };
            let formatted = builder::format_document(
                ui.get_document_text().as_str(),
                action.as_str(),
                width.max(20) as usize,
            );
            ui.set_document_text(formatted.clone().into());
            state.borrow_mut().dirty = true;
            refresh_stats(&ui, &formatted);
            refresh_chrome(
                &ui,
                &state.borrow(),
                &format!("NFO Builder: {} at {} columns", action, width),
            );
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_builder_preview_requested(move |text, style, alignment, box_width, canvas_width| {
            let Some(ui) = weak.upgrade() else { return };
            let _ = (alignment, box_width);
            let preview = builder::build_preview(
                text.as_str(),
                style.as_str(),
                canvas_width.max(20) as usize,
            );
            ui.set_builder_preview(preview.into());
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_builder_scene_example(move || {
            if let Some(ui) = weak.upgrade() {
                let text = templates::BUILDER_SCENE_RELEASE;
                ui.set_builder_text(text.into());
                ui.set_builder_style("double".into());
                ui.set_builder_preview(builder::build_preview(text, "double", 80).into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_builder_add_section(move |text, width, canvas_width, style| {
            let Some(ui) = weak.upgrade() else { return };
            let mut body = text.to_string();
            if !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&builder::section_block(
                style.as_str(),
                width.max(12) as usize,
                canvas_width.max(20) as usize,
            ));
            ui.set_builder_text(body.clone().into());
            ui.set_builder_preview(
                builder::build_preview(&body, style.as_str(), canvas_width.max(20) as usize).into(),
            );
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_builder_align_line(move |text, anchor, cursor, alignment, width| {
            let Some(ui) = weak.upgrade() else { return };
            let (formatted, caret) = if anchor != cursor {
                builder::align_selection(
                    text.as_str(),
                    anchor.max(0) as usize,
                    cursor.max(0) as usize,
                    width.max(20) as usize,
                    alignment.as_str(),
                )
            } else {
                (
                    builder::align_current_line(
                        text.as_str(),
                        cursor.max(0) as usize,
                        width.max(20) as usize,
                        alignment.as_str(),
                    ),
                    cursor.max(0) as usize,
                )
            };
            ui.set_builder_text(formatted.clone().into());
            ui.set_builder_cursor_position(caret as i32);
            ui.set_builder_anchor_position(caret as i32);
            ui.set_builder_preview(
                builder::build_preview(
                    &formatted,
                    ui.get_builder_style().as_str(),
                    ui.get_builder_width().max(20) as usize,
                )
                .into(),
            );
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_builder_box_line(move |text, cursor, width, canvas_width, style, alignment| {
            let Some(ui) = weak.upgrade() else { return };
            let formatted = builder::box_current_line(
                text.as_str(),
                cursor.max(0) as usize,
                width.max(12) as usize,
                canvas_width.max(20) as usize,
                style.as_str(),
                alignment.as_str(),
            );
            ui.set_builder_text(formatted.clone().into());
            ui.set_builder_preview(
                builder::build_preview(
                    &formatted,
                    ui.get_builder_style().as_str(),
                    ui.get_builder_width().max(20) as usize,
                )
                .into(),
            );
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_builder_insert_requested(move |text, style, alignment, box_width, canvas_width| {
            let Some(ui) = weak.upgrade() else { return };
            let _ = (alignment, box_width);
            let built = builder::build_preview(
                text.as_str(),
                style.as_str(),
                canvas_width.max(20) as usize,
            );
            let mut document = ui.get_document_text().to_string();
            if !document.is_empty() && !document.ends_with('\n') {
                document.push('\n');
            }
            if !document.is_empty() {
                document.push('\n');
            }
            document.push_str(&built);
            ui.set_document_text(document.clone().into());
            ui.set_active_tab(0);
            state.borrow_mut().dirty = true;
            refresh_stats(&ui, &document);
            refresh_chrome(&ui, &state.borrow(), "visual box inserted as NFO text");
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_export_requested(move |format, font_family| {
            let Some(ui) = weak.upgrade() else { return };
            let ext = if format == "webp" { "webp" } else { "png" };
            if let Some(path) = FileDialog::new()
                .set_file_name(format!("nfo-export.{ext}"))
                .add_filter(ext.to_uppercase(), &[ext])
                .save_file()
            {
                match export::render(
                    ui.get_document_text().as_str(),
                    &path,
                    ext,
                    font_family.as_str(),
                ) {
                    Ok(()) => ui.set_status_text(format!("exported {}", path.display()).into()),
                    Err(error) => set_error(&ui, error),
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_quit_requested(move || {
            if let Some(ui) = weak.upgrade() {
                let _ = ui.hide();
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let gpg_keys = gpg_keys.clone();
        ui.on_sign_requested(move || {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(path) = FileDialog::new()
                .set_file_name("document.nfo.asc")
                .add_filter("ASCII-armored signature", &["asc"])
                .save_file()
            {
                let (bytes, _) = encode_document(ui.get_document_text().as_str(), &state.borrow());
                let fingerprint = gpg_keys
                    .borrow()
                    .get(ui.get_selected_gpg_key().max(0) as usize)
                    .map(|key| key.fingerprint.clone());
                match gpg::sign_detached(&bytes, &path, fingerprint.as_deref()) {
                    Ok(()) => ui.set_status_text(format!("signed → {}", path.display()).into()),
                    Err(error) => set_error(&ui, error),
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let gpg_keys = gpg_keys.clone();
        ui.on_refresh_gpg_keys(move || {
            if let Some(ui) = weak.upgrade() {
                refresh_gpg_keys(&ui, &gpg_keys);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let gpg_keys = gpg_keys.clone();
        ui.on_set_default_gpg_key(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            let keys = gpg_keys.borrow();
            let Some(key) = keys.get(index.max(0) as usize) else {
                ui.set_status_text("no secret GPG key is available".into());
                return;
            };
            match config::save_default_gpg_key(&key.fingerprint) {
                Ok(()) => {
                    ui.set_gpg_status(format!("GPG default: {}", key.label).into());
                    ui.set_status_text(format!("default GPG key saved: {}", key.label).into());
                }
                Err(error) => set_error(&ui, error),
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_verify_requested(move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(document_path) = state.borrow().path.clone() else {
                ui.set_status_text("save the document before verification".into());
                return;
            };
            if let Some(signature_path) = FileDialog::new()
                .add_filter("GPG signature", &["asc", "sig"])
                .pick_file()
            {
                match gpg::verify(&signature_path, &document_path) {
                    Ok(message) => ui.set_status_text(message.into()),
                    Err(error) => set_error(&ui, error),
                }
            }
        });
    }

    ui.run()?;
    Ok(())
}

fn install_save_handler(ui: &AppWindow, state: Rc<RefCell<Document>>, save_as: bool) {
    let weak = ui.as_weak();
    let handler = move || {
        let Some(ui) = weak.upgrade() else { return };
        let path = if save_as || state.borrow().path.is_none() {
            FileDialog::new()
                .set_file_name("document.nfo")
                .add_filter("NFO file", &["nfo"])
                .add_filter("Text file", &["txt", "diz", "asc"])
                .save_file()
        } else {
            state.borrow().path.clone()
        };
        let Some(path) = path else { return };
        match save_document(&path, ui.get_document_text().as_str(), &state.borrow()) {
            Ok(replacements) => {
                let mut doc = state.borrow_mut();
                doc.path = Some(path);
                doc.dirty = false;
                let message = if replacements == 0 {
                    "saved".to_string()
                } else {
                    format!("saved with {replacements} unsupported character(s) replaced by ?")
                };
                refresh_chrome(&ui, &doc, &message);
            }
            Err(error) => set_error(&ui, error),
        }
    };
    if save_as {
        ui.on_save_as_requested(handler);
    } else {
        ui.on_save_requested(handler);
    }
}

fn open_document(path: &PathBuf, encoding: Encoding) -> Result<(String, LineEnding)> {
    let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
    let ending = if bytes.windows(2).any(|pair| pair == b"\r\n") {
        LineEnding::CrLf
    } else if bytes.contains(&b'\n') {
        LineEnding::Lf
    } else {
        LineEnding::Cr
    };
    // Plain text files are overwhelmingly UTF-8/UTF-16. Treating a UTF-8 .txt
    // as CP437 turns every non-ASCII sequence into unrelated box glyphs and
    // can overwhelm the native text renderer on real-world files.
    let decoded = if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
    {
        decode_text_file(&bytes)?
    } else {
        match encoding {
            Encoding::Cp437 => cp437::decode(&bytes),
            Encoding::Utf8 => String::from_utf8(bytes).context("file is not valid UTF-8")?,
            Encoding::Windows1252 => WINDOWS_1252.decode(&bytes).0.into_owned(),
        }
    };
    Ok((normalize_newlines(&decoded), ending))
}

fn decode_text_file(bytes: &[u8]) -> Result<String> {
    if let Some(bytes) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8(bytes.to_vec())
            .context("text file has an invalid UTF-8 sequence");
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let words = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        anyhow::ensure!(
            bytes.len() % 2 == 0,
            "text file has an incomplete UTF-16LE character"
        );
        return String::from_utf16(&words).context("text file has invalid UTF-16LE");
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let words = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        anyhow::ensure!(
            bytes.len() % 2 == 0,
            "text file has an incomplete UTF-16BE character"
        );
        return String::from_utf16(&words).context("text file has invalid UTF-16BE");
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_owned()),
        Err(_) => Ok(WINDOWS_1252.decode(bytes).0.into_owned()),
    }
}

fn save_document(path: &PathBuf, text: &str, doc: &Document) -> Result<usize> {
    let (bytes, replacements) = encode_document(text, doc);
    fs::write(path, bytes).with_context(|| format!("could not write {}", path.display()))?;
    Ok(replacements)
}

fn encode_document(text: &str, doc: &Document) -> (Vec<u8>, usize) {
    let ending = match doc.line_ending {
        LineEnding::CrLf => "\r\n",
        LineEnding::Lf => "\n",
        LineEnding::Cr => "\r",
    };
    let normalized = normalize_newlines(text).replace('\n', ending);
    let (bytes, replacements) = match doc.encoding {
        Encoding::Cp437 => cp437::encode(&normalized),
        Encoding::Utf8 => (normalized.into_bytes(), 0),
        Encoding::Windows1252 => {
            let (encoded, _, had_errors) = WINDOWS_1252.encode(&normalized);
            (encoded.into_owned(), usize::from(had_errors))
        }
    };
    (bytes, replacements)
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn refresh_stats(ui: &AppWindow, text: &str) {
    ui.set_char_count(text.chars().count() as i32);
    ui.set_line_count(if text.is_empty() {
        1
    } else {
        text.lines().count() as i32
    });
    ui.set_widest_line(
        text.lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as i32,
    );
}

fn refresh_chrome(ui: &AppWindow, doc: &Document, message: &str) {
    let name = doc
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("untitled.nfo");
    ui.set_file_name(if doc.dirty {
        format!("• {name}").into()
    } else {
        name.into()
    });
    ui.set_encoding_name(doc.encoding.name().into());
    ui.set_line_ending_name(doc.line_ending.name().into());
    ui.set_status_text(message.into());
}

fn set_error(ui: &AppWindow, error: impl std::fmt::Display) {
    ui.set_status_text(SharedString::from(format!("error: {error}")));
}

fn refresh_gpg_keys(ui: &AppWindow, key_store: &Rc<RefCell<Vec<gpg::SigningKey>>>) {
    match gpg::list_secret_keys() {
        Ok(keys) if !keys.is_empty() => {
            let saved = config::load_default_gpg_key();
            let selected = saved
                .as_ref()
                .and_then(|fingerprint| keys.iter().position(|key| &key.fingerprint == fingerprint))
                .unwrap_or(0);
            let labels: Vec<SharedString> =
                keys.iter().map(|key| key.label.clone().into()).collect();
            let status = if saved.is_some() {
                format!("GPG default: {}", keys[selected].label)
            } else {
                format!("GPG: {} secret key(s)", keys.len())
            };
            *key_store.borrow_mut() = keys;
            ui.set_gpg_keys(ModelRc::new(VecModel::from(labels)));
            ui.set_selected_gpg_key(selected as i32);
            ui.set_gpg_status(status.into());
        }
        Ok(_) => {
            key_store.borrow_mut().clear();
            ui.set_gpg_keys(ModelRc::new(VecModel::from(vec![SharedString::from(
                "System default (no secret keys found)",
            )])));
            ui.set_selected_gpg_key(0);
            ui.set_gpg_status("GPG: no secret keys found".into());
        }
        Err(error) => {
            key_store.borrow_mut().clear();
            ui.set_gpg_keys(ModelRc::new(VecModel::from(vec![SharedString::from(
                "GPG unavailable",
            )])));
            ui.set_selected_gpg_key(0);
            ui.set_gpg_status(format!("GPG unavailable: {error}").into());
        }
    }
}
