//! A datasheet PDF for the footprint review, read and drawn by hayro (pure Rust) on a thread of its own: the page
//! sizes first, then one picture of the page asked for at a time, and the text of every page for the page search
//! while nothing is asked. Page coordinates are PDF points (1/72 inch), y down, as the page is shown.

use hayro::hayro_interpret::font::{FontData, FontQuery, StandardFont};
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context as HayroContext, Device, DrawMode, DrawProps, Image,
    ImageDrawProps, InterpreterCache, InterpreterSettings, SoftMask, font::GlyphRun,
    interpret_page,
};
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::page::Page;
use hayro::kurbo::{BezPath, Rect};
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

/// What the reader thread sends back
enum Reply {
    /// The file is read: each page's size in points
    Opened(Vec<[f32; 2]>),
    Failed(String),
    /// A page's picture
    Picture {
        page: usize,
        image: egui::ColorImage,
    },
    /// A page's text, for the page search
    Text {
        page: usize,
        text: String,
    },
    /// Nothing left to do until something is asked
    Idle,
}

/// A page's picture and the scale it was drawn at
pub struct Picture {
    pub page: usize,
    pub texture: egui::TextureHandle,
}

/// A PDF file open for the review
pub struct PdfDoc {
    pub path: PathBuf,
    /// Each page's size in points, once read
    pub pages: Option<Vec<[f32; 2]>>,
    pub error: Option<String>,
    /// Each page's text once read, `None` until then
    pub texts: Vec<Option<String>>,
    pub picture: Option<Picture>,
    /// The page and scale asked for last, so a picture is asked once
    asked: Option<(usize, f32)>,
    requests: Sender<(usize, f32)>,
    replies: Receiver<Reply>,
    idle: bool,
}

/// Where hayro finds the PDF standard fonts (Helvetica, Times, Courier) that a datasheet uses without embedding
/// them: their Liberation look-alikes in one of `dirs`, else egui's own fonts (Ubuntu Light, Hack), so the text is
/// there to draw and to search on any PC
pub fn font_settings(dirs: &[PathBuf]) -> InterpreterSettings {
    let dirs = dirs.to_vec();
    let egui_fonts = egui::FontDefinitions::default().font_data;
    let builtin = |name: &str| -> Option<FontData> {
        let bytes: Vec<u8> = egui_fonts.get(name)?.font.to_vec();
        Some(Arc::new(bytes))
    };
    let (sans, mono) = (builtin("Ubuntu-Light"), builtin("Hack"));
    let resolve = move |s: StandardFont| -> Option<(FontData, u32)> {
        let name = match s {
            StandardFont::Helvetica => "LiberationSans-Regular.ttf",
            StandardFont::HelveticaBold => "LiberationSans-Bold.ttf",
            StandardFont::HelveticaOblique => "LiberationSans-Italic.ttf",
            StandardFont::HelveticaBoldOblique => "LiberationSans-BoldItalic.ttf",
            StandardFont::Courier => "LiberationMono-Regular.ttf",
            StandardFont::CourierBold => "LiberationMono-Bold.ttf",
            StandardFont::CourierOblique => "LiberationMono-Italic.ttf",
            StandardFont::CourierBoldOblique => "LiberationMono-BoldItalic.ttf",
            StandardFont::TimesRoman => "LiberationSerif-Regular.ttf",
            StandardFont::TimesBold => "LiberationSerif-Bold.ttf",
            StandardFont::TimesItalic => "LiberationSerif-Italic.ttf",
            StandardFont::TimesBoldItalic => "LiberationSerif-BoldItalic.ttf",
            StandardFont::ZapfDingBats | StandardFont::Symbol => return None,
        };
        match dirs.iter().find_map(|d| std::fs::read(d.join(name)).ok()) {
            Some(data) => Some((Arc::new(data) as FontData, 0)),
            None if name.starts_with("LiberationMono") => mono.clone().map(|f| (f, 0)),
            None => sans.clone().map(|f| (f, 0)),
        }
    };
    InterpreterSettings {
        font_resolver: Arc::new(move |q| match q {
            FontQuery::Standard(s) => resolve(*s),
            FontQuery::Fallback(f) => resolve(f.pick_standard_font()),
        }),
        ..Default::default()
    }
}

impl PdfDoc {
    /// Starts reading `path` on a thread of its own, which wakes `ctx` when it has something;
    /// [`poll`](Self::poll) takes it in
    pub fn open(ctx: &egui::Context, path: &Path, font_dirs: &[PathBuf]) -> PdfDoc {
        let (req_tx, req_rx) = channel();
        let (rep_tx, rep_rx) = channel();
        let file = path.to_path_buf();
        let settings = font_settings(font_dirs);
        let wake = ctx.clone();
        std::thread::Builder::new()
            .name("pdf reader".into())
            .spawn(move || {
                reader(
                    file,
                    settings,
                    req_rx,
                    &Waking {
                        tx: rep_tx,
                        ctx: wake,
                    },
                )
            })
            .expect("a thread starts");
        PdfDoc {
            path: path.to_path_buf(),
            pages: None,
            error: None,
            texts: Vec::new(),
            picture: None,
            asked: None,
            requests: req_tx,
            replies: rep_rx,
            idle: false,
        }
    }

    /// Takes in what the reader thread sent; true when something changed
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;
        while let Ok(r) = self.replies.try_recv() {
            changed = true;
            self.take(ctx, r);
        }
        changed
    }

    /// Waits until the reader thread has done everything asked so far and read every page's text (tests, and a
    /// search that needs every page)
    #[cfg(test)]
    pub fn wait_idle(&mut self, ctx: &egui::Context) {
        while !self.idle && self.error.is_none() {
            match self.replies.recv() {
                Ok(r) => self.take(ctx, r),
                Err(_) => break,
            }
        }
    }

    fn take(&mut self, ctx: &egui::Context, r: Reply) {
        self.idle = matches!(r, Reply::Idle);
        match r {
            Reply::Opened(pages) => {
                self.texts = vec![None; pages.len()];
                self.pages = Some(pages);
            }
            Reply::Failed(e) => self.error = Some(e),
            Reply::Picture { page, image } => {
                let texture = ctx.load_texture(
                    format!("pdf page {page}"),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                self.picture = Some(Picture { page, texture });
            }
            Reply::Text { page, text } => {
                if let Some(t) = self.texts.get_mut(page) {
                    *t = Some(text);
                }
            }
            Reply::Idle => {}
        }
    }

    pub fn page_count(&self) -> usize {
        self.pages.as_ref().map_or(0, Vec::len)
    }

    /// A page's size in points
    pub fn page_size(&self, page: usize) -> Option<[f32; 2]> {
        self.pages.as_ref()?.get(page).copied()
    }

    /// Asks for `page` drawn at `scale` pixels per point, at most as large as a texture may be on `ctx`, unless
    /// that is what was asked last
    pub fn want(&mut self, ctx: &egui::Context, page: usize, scale: f32) {
        let Some([w, h]) = self.page_size(page) else {
            return;
        };
        let max_side = ctx.input(|i| i.max_texture_side) as f32;
        let scale = scale.min(max_side / w.max(h).max(1.0)).max(0.05);
        if self.asked == Some((page, scale)) {
            return;
        }
        self.asked = Some((page, scale));
        self.idle = false;
        let _ = self.requests.send((page, scale));
    }

    /// Every page's text has been read
    pub fn texts_done(&self) -> bool {
        self.pages.is_some() && self.texts.iter().all(Option::is_some)
    }

    /// The pages whose text has any of `words`, best first: by how many of them it has, then by the place of the
    /// first one in `words` (earlier words count more), then by page. Case, spaces and punctuation don't matter.
    pub fn find_pages(&self, words: &[String]) -> Vec<PageHit> {
        let words: Vec<String> = words.iter().map(|w| normalize(w)).collect();
        let mut hits: Vec<PageHit> = self
            .texts
            .iter()
            .enumerate()
            .filter_map(|(page, t)| {
                let t = normalize(t.as_deref()?);
                let found: Vec<usize> = words
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| !w.is_empty() && t.contains(w.as_str()))
                    .map(|(i, _)| i)
                    .collect();
                (!found.is_empty()).then_some(PageHit { page, words: found })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.words
                .len()
                .cmp(&a.words.len())
                .then(a.words[0].cmp(&b.words[0]))
                .then(a.page.cmp(&b.page))
        });
        hits
    }
}

/// A page the page search found and which of the search words it has (their places in the list)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageHit {
    pub page: usize,
    pub words: Vec<usize>,
}

/// Lower case letters and digits only
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The reply channel, waking the UI with each reply
struct Waking {
    tx: Sender<Reply>,
    ctx: egui::Context,
}

impl Waking {
    fn send(&self, r: Reply) -> Result<(), ()> {
        self.tx.send(r).map_err(|_| ())?;
        self.ctx.request_repaint();
        Ok(())
    }
}

fn reader(
    file: PathBuf,
    settings: InterpreterSettings,
    requests: Receiver<(usize, f32)>,
    replies: &Waking,
) {
    let pdf = match std::fs::read(&file)
        .map_err(|e| e.to_string())
        .and_then(|d| Pdf::new(Arc::new(d)).map_err(|e| format!("{e:?}")))
    {
        Ok(p) => p,
        Err(e) => {
            let _ = replies.send(Reply::Failed(format!("can't read {}: {e}", file.display())));
            return;
        }
    };
    let pages = pdf.pages();
    let sizes = pages
        .iter()
        .map(|p| {
            let (w, h) = p.render_dimensions();
            [w, h]
        })
        .collect();
    if replies.send(Reply::Opened(sizes)).is_err() {
        return;
    }
    let cache = RenderCache::new();
    let text_cache = InterpreterCache::new();
    let mut next_text = 0;
    let mut pending: Option<(usize, f32)> = None;
    loop {
        // The newest request first; the text of the next page while there is none
        loop {
            match requests.try_recv() {
                Ok(r) => pending = Some(r),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let reply = if let Some((page, scale)) = pending.take() {
            let Some(p) = pages.get(page) else { continue };
            let image = draw(p, &cache, &settings, scale);
            Reply::Picture { page, image }
        } else if next_text < pages.len() {
            let text = page_text(&pages[next_text], &text_cache, &settings);
            next_text += 1;
            Reply::Text {
                page: next_text - 1,
                text,
            }
        } else {
            if replies.send(Reply::Idle).is_err() {
                return;
            }
            match requests.recv() {
                Ok(r) => pending = Some(r),
                Err(_) => return,
            }
            continue;
        };
        if replies.send(reply).is_err() {
            return;
        }
    }
}

/// A page drawn at `scale` pixels per point on white
fn draw<'a>(
    p: &'a Page<'a>,
    cache: &RenderCache<'a>,
    settings: &InterpreterSettings,
    scale: f32,
) -> egui::ColorImage {
    let pixmap = hayro::render(
        p,
        cache,
        settings,
        &RenderSettings::default(),
        &PixmapSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
        },
    );
    let size = [pixmap.width() as usize, pixmap.height() as usize];
    egui::ColorImage::from_rgba_premultiplied(size, pixmap.data_as_u8_slice())
}

/// The text of a page, for the page search
fn page_text<'a>(
    p: &Page<'a>,
    cache: &InterpreterCache<'a>,
    settings: &InterpreterSettings,
) -> String {
    let mut device = TextDevice::default();
    let (w, h) = p.render_dimensions();
    let mut cx = HayroContext::new(
        p.initial_transform(true).to_kurbo(),
        Rect::new(0.0, 0.0, w as f64, h as f64),
        cache,
        p.xref(),
        settings.clone(),
    );
    interpret_page(p, &mut cx, &mut device);
    device.text
}

/// Collects a page's text: the glyphs' characters, a line break between runs
#[derive(Default)]
struct TextDevice {
    text: String,
}

impl<'a> Device<'a> for TextDevice {
    fn draw_path(&mut self, _: &BezPath, _: DrawProps<'a>, _: &DrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_glyph_run(&mut self, run: &GlyphRun<'_, 'a>, _: DrawProps<'a>, _: &DrawMode) {
        for g in run.glyphs() {
            match g.as_unicode() {
                Some(BfString::Char(c)) => self.text.push(c),
                Some(BfString::String(s)) => self.text.push_str(&s),
                None => {}
            }
        }
        self.text.push('\n');
    }
    fn draw_image(&mut self, _: Image<'a, '_>, _: ImageDrawProps<'a>) {}
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::super::testpdf::{TestPage, write};
    use super::*;

    /// A written PDF: its pages' sizes, a picture of the size asked, every page's text, and the page search
    /// ranking the land pattern page first
    #[test]
    fn read_draw_and_search() {
        let dir = std::env::temp_dir().join(format!("berc-pdf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.pdf");
        let text = |y: f32, t: &str| (50.0, y, 12.0, t.to_string());
        let pages = [
            TestPage {
                texts: vec![text(60.0, "Features"), text(80.0, "Package information")],
                ..Default::default()
            },
            TestPage {
                texts: vec![
                    text(60.0, "Recommended Land Pattern"),
                    text(80.0, "Package outline"),
                ],
                lines: vec![[100.0, 100.0, 200.0, 100.0]],
                rects: vec![[100.0, 120.0, 20.0, 10.0]],
            },
            TestPage::default(),
        ];
        std::fs::write(&file, write([300.0, 400.0], &pages)).unwrap();
        let ctx = egui::Context::default();
        let mut doc = PdfDoc::open(&ctx, &file, &[]);
        doc.wait_idle(&ctx);
        doc.want(&ctx, 1, 2.0);
        doc.wait_idle(&ctx);
        assert_eq!(doc.error, None);
        assert_eq!(doc.pages.as_deref(), Some(&[[300.0, 400.0]; 3][..]));
        let pic = doc.picture.as_ref().unwrap();
        assert_eq!(pic.page, 1);
        assert_eq!(pic.texture.size(), [600, 800]);
        assert!(doc.texts_done());
        assert!(
            doc.texts[1].as_deref().unwrap().contains("Land Pattern"),
            "{:?}",
            doc.texts
        );
        let words = ["land pattern", "package"].map(String::from);
        let hits = doc.find_pages(&words);
        assert_eq!(
            hits,
            [
                PageHit {
                    page: 1,
                    words: vec![0, 1]
                },
                PageHit {
                    page: 0,
                    words: vec![1]
                }
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod real {
    use super::*;

    /// A look at a real datasheet: BERC_PDF names one (not run without it)
    #[test]
    fn real_datasheet() {
        let Some(file) = std::env::var_os("BERC_PDF") else {
            eprintln!("BERC_PDF not set: no real datasheet read");
            return;
        };
        let ctx = egui::Context::default();
        let t = std::time::Instant::now();
        let mut doc = PdfDoc::open(
            &ctx,
            Path::new(&file),
            &[PathBuf::from("/usr/share/fonts/liberation")],
        );
        doc.wait_idle(&ctx);
        doc.want(&ctx, 0, 1.0);
        doc.wait_idle(&ctx);
        eprintln!(
            "{:?} {:?} pages, {:?}",
            doc.error,
            doc.page_count(),
            t.elapsed()
        );
        let words = [
            "land pattern",
            "footprint",
            "soldering",
            "package outline",
            "mounting pad",
        ]
        .map(String::from);
        eprintln!("{:?}", doc.find_pages(&words));
    }
}
