//! A datasheet PDF for the footprint review, read and drawn by hayro (pure Rust) on a thread of its own: the page
//! sizes first, then one picture of the page asked for at a time, and the text of every page for the page search
//! while nothing is asked, with its lines and words for the ruler ([`PageGeometry`]). Page coordinates are PDF
//! points (1/72 inch), y down, as the page is shown.

use super::geometry::{PageGeometry, Seg, Word};
use hayro::hayro_interpret::font::{FontData, FontQuery, StandardFont};
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context as HayroContext, Device, DrawMode, DrawProps, Image,
    ImageDrawProps, InterpreterCache, InterpreterSettings, SoftMask, font::Glyph, font::GlyphRun,
    interpret_page,
};
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::page::Page;
use hayro::kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};
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
    /// A page's text, for the page search, and its lines and words, for the ruler
    Text {
        page: usize,
        text: String,
        geometry: Arc<PageGeometry>,
    },
    /// Nothing left to do until something is asked
    Idle,
}

/// A page's picture: the texture egui draws, the pixels the blend pass reads and a generation that changes with them
pub struct Picture {
    pub page: usize,
    pub texture: egui::TextureHandle,
    pub image: Arc<egui::ColorImage>,
    pub generation: u64,
}

/// A unique generation for a picture or its inverted copy, so the blend pass' texture cache re-uploads only when
/// what it reads has changed
fn next_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The current picture with its colours inverted (white-ish on black), computed once per picture and cached
struct Inverted {
    /// The picture's own generation this was computed from
    source: u64,
    generation: u64,
    image: Arc<egui::ColorImage>,
    texture: egui::TextureHandle,
}

/// A PDF file open for the review
pub struct PdfDoc {
    pub path: PathBuf,
    /// Each page's size in points, once read
    pub pages: Option<Vec<[f32; 2]>>,
    pub error: Option<String>,
    /// Each page's text once read, `None` until then
    pub texts: Vec<Option<String>>,
    /// Each page's lines and words once read, `None` until then
    pub geometry: Vec<Option<Arc<PageGeometry>>>,
    pub picture: Option<Picture>,
    inverted: Option<Inverted>,
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
            geometry: Vec::new(),
            picture: None,
            inverted: None,
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
                self.geometry = vec![None; pages.len()];
                self.pages = Some(pages);
            }
            Reply::Failed(e) => self.error = Some(e),
            Reply::Picture { page, image } => {
                let image = Arc::new(image);
                let texture = ctx.load_texture(
                    format!("pdf page {page}"),
                    image.clone(),
                    egui::TextureOptions::LINEAR,
                );
                // Unique across documents: the blend pass uploads a picture again when this changes
                let generation = next_generation();
                self.picture = Some(Picture {
                    page,
                    texture,
                    image,
                    generation,
                });
            }
            Reply::Text {
                page,
                text,
                geometry,
            } => {
                if let Some(t) = self.texts.get_mut(page) {
                    *t = Some(text);
                }
                if let Some(g) = self.geometry.get_mut(page) {
                    *g = Some(geometry);
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

    /// A page's lines and words, once read
    pub fn geometry(&self, page: usize) -> Option<&PageGeometry> {
        self.geometry.get(page)?.as_deref()
    }

    /// Every page's text has been read
    pub fn texts_done(&self) -> bool {
        self.pages.is_some() && self.texts.iter().all(Option::is_some)
    }

    /// The current picture with its colours inverted (white-ish on black), its own generation (for the blend
    /// pass' texture cache) and the texture id egui draws; computed once per picture and cached, none without one
    pub fn inverted(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<(&Arc<egui::ColorImage>, egui::TextureId, u64)> {
        let pic = self.picture.as_ref()?;
        if self.inverted.as_ref().map(|i| i.source) != Some(pic.generation) {
            let pixels = pic
                .image
                .pixels
                .iter()
                .map(|c| {
                    egui::Color32::from_rgba_premultiplied(
                        255 - c.r(),
                        255 - c.g(),
                        255 - c.b(),
                        c.a(),
                    )
                })
                .collect();
            let image = Arc::new(egui::ColorImage {
                size: pic.image.size,
                source_size: pic.image.source_size,
                pixels,
            });
            let texture = ctx.load_texture(
                format!("pdf page {} inverted", pic.page),
                image.clone(),
                egui::TextureOptions::LINEAR,
            );
            self.inverted = Some(Inverted {
                source: pic.generation,
                generation: next_generation(),
                image,
                texture,
            });
        }
        self.inverted
            .as_ref()
            .map(|i| (&i.image, i.texture.id(), i.generation))
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
            let (text, geometry) = page_text(&pages[next_text], &text_cache, &settings);
            next_text += 1;
            Reply::Text {
                page: next_text - 1,
                text,
                geometry: Arc::new(geometry),
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

/// The text of a page, for the page search, and its lines and words, for the ruler
fn page_text<'a>(
    p: &Page<'a>,
    cache: &InterpreterCache<'a>,
    settings: &InterpreterSettings,
) -> (String, PageGeometry) {
    let mut device = PageDevice::default();
    let (w, h) = p.render_dimensions();
    let mut cx = HayroContext::new(
        p.initial_transform(true).to_kurbo(),
        Rect::new(0.0, 0.0, w as f64, h as f64),
        cache,
        p.xref(),
        settings.clone(),
    );
    interpret_page(p, &mut cx, &mut device);
    device.word_done();
    (device.text, device.geometry)
}

/// The most line pieces kept of a page, so a page of hatching or outlined text stays small
const MAX_SEGS: usize = 200_000;

/// A filled rectangle at most this wide (points) and three times as long is a line, as some drawings draw them
const THIN_RECT: f64 = 2.0;

/// Collects a page's text (the glyphs' characters, a line break between runs), the straight pieces of its lines
/// (stroked paths' centre lines with their width, thin filled rectangles' centre lines, other filled shapes'
/// edges) and its words with the boxes of their glyphs' ink, in page points
#[derive(Default)]
struct PageDevice {
    text: String,
    geometry: PageGeometry,
    /// The word being read: its text, box, and its last glyph's box, writing direction and em size
    word: Option<WordAcc>,
}

struct WordAcc {
    text: String,
    bbox: Rect,
    /// Where the next glyph of the word starts: the last one's origin moved on by its advance
    pen: Point,
    dir: [f64; 2],
    em: f64,
}

impl PageDevice {
    fn seg(&mut self, a: Point, b: Point, width: f64) {
        if self.geometry.segs.len() < MAX_SEGS && a.distance(b) > 1e-3 {
            self.geometry.segs.push(Seg {
                a: [a.x as f32, a.y as f32],
                b: [b.x as f32, b.y as f32],
                width: width as f32,
            });
        }
    }

    /// The word being read is complete
    fn word_done(&mut self) {
        if let Some(w) = self.word.take() {
            self.geometry.words.push(Word {
                text: w.text,
                min: [w.bbox.x0 as f32, w.bbox.y0 as f32],
                max: [w.bbox.x1 as f32, w.bbox.y1 as f32],
            });
        }
    }

    /// A glyph `c`: its ink's box on the page, its origin and where the next glyph starts (`pen`), its writing
    /// direction and em size. It goes on the word being read when it starts where that word's last glyph ended,
    /// else starts a word; a glyph without ink (a space) ends one.
    fn glyph(
        &mut self,
        c: &str,
        bbox: Option<Rect>,
        origin: Point,
        pen: Point,
        dir: [f64; 2],
        em: f64,
    ) {
        let Some(bbox) = bbox.filter(|_| !c.trim().is_empty()) else {
            self.word_done();
            return;
        };
        let follows = self
            .word
            .as_ref()
            .is_some_and(|w| dir == w.dir && origin.distance(w.pen) < 0.15 * w.em.max(em));
        if !follows {
            self.word_done();
            self.word = Some(WordAcc {
                text: String::new(),
                bbox,
                pen,
                dir,
                em,
            });
        }
        let w = self.word.as_mut().expect("a word");
        w.text.push_str(c);
        w.bbox = w.bbox.union(bbox);
        w.pen = pen;
    }
}

/// The corners of a closed path of straight lines that is a rectangle (any angle), when it is one
fn rect_corners(pts: &[Point]) -> Option<[Point; 4]> {
    let pts = match pts {
        [a, .., z] if pts.len() == 5 && a.distance(*z) < 1e-6 => &pts[..4],
        _ => pts,
    };
    let [p0, p1, p2, p3] = <[Point; 4]>::try_from(pts).ok()?;
    let (e0, e1, e2, e3) = (p1 - p0, p2 - p1, p3 - p2, p0 - p3);
    let tol = 1e-3 * (e0.hypot() + e1.hypot()).max(1e-6);
    let square = e0.dot(e1).abs() <= tol * e0.hypot().max(e1.hypot());
    ((e0 + e2).hypot() <= tol && (e1 + e3).hypot() <= tol && square).then_some([p0, p1, p2, p3])
}

impl<'a> Device<'a> for PageDevice {
    fn draw_path(&mut self, path: &BezPath, props: DrawProps<'a>, mode: &DrawMode) {
        let t: Affine = props.transform;
        let (fill, stroke) = match mode {
            DrawMode::Fill(_) => (true, None),
            DrawMode::Stroke(s) => (false, Some(s.line_width)),
            DrawMode::FillAndStroke(_, s) => (true, Some(s.line_width)),
            DrawMode::Invisible => return,
        };
        let scale = t.determinant().abs().sqrt();
        // Each subpath: its points while it has only straight lines, the stroked ones kept as they come
        let mut sub: Vec<Point> = Vec::new();
        let mut straight = true;
        let mut start = Point::ZERO;
        let mut at = Point::ZERO;
        let flush = |dev: &mut Self, sub: &mut Vec<Point>, straight: bool| {
            if fill && stroke.is_none() && straight && sub.len() >= 3 {
                match rect_corners(sub) {
                    Some([p0, p1, p2, p3])
                        if {
                            let (a, b) = (p0.distance(p1), p1.distance(p2));
                            a.min(b) <= THIN_RECT && a.max(b) >= 3.0 * a.min(b)
                        } =>
                    {
                        let (a, b) = (p0.distance(p1), p1.distance(p2));
                        if a >= b {
                            dev.seg(p0.midpoint(p3), p1.midpoint(p2), b);
                        } else {
                            dev.seg(p0.midpoint(p1), p3.midpoint(p2), a);
                        }
                    }
                    _ => {
                        for i in 0..sub.len() {
                            dev.seg(sub[i], sub[(i + 1) % sub.len()], 0.0);
                        }
                    }
                }
            }
            sub.clear();
        };
        for el in path.elements() {
            match *el {
                PathEl::MoveTo(p) => {
                    flush(self, &mut sub, straight);
                    straight = true;
                    start = t * p;
                    at = start;
                    sub.push(at);
                }
                PathEl::LineTo(p) => {
                    let q = t * p;
                    if let Some(w) = stroke {
                        self.seg(at, q, w as f64 * scale);
                    }
                    at = q;
                    sub.push(q);
                }
                PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                    straight = false;
                    at = t * p;
                    sub.push(at);
                }
                PathEl::ClosePath => {
                    if let Some(w) = stroke {
                        self.seg(at, start, w as f64 * scale);
                    }
                    at = start;
                }
            }
        }
        flush(self, &mut sub, straight);
    }
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_glyph_run(&mut self, run: &GlyphRun<'_, 'a>, props: DrawProps<'a>, _: &DrawMode) {
        for g in run.glyphs() {
            let c = match g.as_unicode() {
                Some(BfString::Char(c)) => c.to_string(),
                Some(BfString::String(s)) => s.to_string(),
                None => String::new(),
            };
            self.text.push_str(&c);
            let t = props.transform * g.transform();
            let [a, b, _, _, _, _] = t.as_coeffs();
            let len = a.hypot(b).max(1e-12);
            // Outlines are in units of 1/1000 em
            let em = t.determinant().abs().sqrt() * 1000.0;
            let (ink, advance) = match &**g {
                Glyph::Outline(o) => {
                    let r = o.outline().bounding_box();
                    ((r.area() > 0.0).then_some(r), o.advance_width())
                }
                Glyph::Type3(_) => (None, None),
            };
            // Without its advance, the next glyph starts where this one's ink ends
            let advance = advance.map_or_else(|| ink.map_or(0.0, |r| r.x1), f64::from);
            self.glyph(
                &c,
                ink.map(|r| t.transform_rect_bbox(r)),
                t * Point::ZERO,
                t * Point::new(advance, 0.0),
                [a / len, b / len],
                em,
            );
        }
        // A word may go on in the next run (a number shown in pieces); the next glyph's place decides
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
                ..Default::default()
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

    /// A page's lines and words from the PDF's own paths and glyphs: stroked lines' centre lines with their
    /// widths, a thin filled rectangle as the line it draws, a large one as its edges; words split at spaces, a
    /// number shown in two pieces read as one word
    #[test]
    fn lines_and_words() {
        let dir = std::env::temp_dir().join(format!("berc-pdf-geom-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("g.pdf");
        let text = |x: f32, y: f32, t: &str| (x, y, 10.0, t.to_string());
        let page = TestPage {
            texts: vec![
                text(140.0, 190.0, "2.00"),
                text(50.0, 300.0, "4X 0.50"),
                // 1.27 in two pieces, the second right after the first
                text(200.0, 300.0, "1"),
                text(205.56, 300.0, ".27"),
            ],
            lines: vec![[100.0, 200.0, 200.0, 200.0]],
            wide: vec![([100.0, 150.0, 100.0, 250.0], 2.0)],
            rects: vec![[150.0, 100.0, 50.0, 0.8], [20.0, 20.0, 40.0, 20.0]],
        };
        std::fs::write(&file, write([300.0, 400.0], &[page])).unwrap();
        let ctx = egui::Context::default();
        let mut doc = PdfDoc::open(&ctx, &file, &[]);
        doc.wait_idle(&ctx);
        let g = doc.geometry(0).expect("the page's lines and words").clone();
        let has = |a: [f32; 2], b: [f32; 2], w: f32| {
            g.segs.iter().any(|s| {
                let close = |p: [f32; 2], q: [f32; 2]| {
                    (p[0] - q[0]).abs() < 0.01 && (p[1] - q[1]).abs() < 0.01
                };
                ((close(s.a, a) && close(s.b, b)) || (close(s.a, b) && close(s.b, a)))
                    && (s.width - w).abs() < 0.01
            })
        };
        assert!(has([100.0, 200.0], [200.0, 200.0], 0.5), "{:?}", g.segs);
        assert!(has([100.0, 150.0], [100.0, 250.0], 2.0), "{:?}", g.segs);
        assert!(
            has([150.0, 100.4], [200.0, 100.4], 0.8),
            "a thin rectangle is a line: {:?}",
            g.segs
        );
        assert!(
            has([20.0, 20.0], [60.0, 20.0], 0.0),
            "a large one its edges: {:?}",
            g.segs
        );
        assert_eq!(g.segs.len(), 3 + 4, "{:?}", g.segs);
        let words: Vec<&str> = g.words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(words, ["2.00", "4X", "0.50", "1.27"]);
        let two = &g.words[0];
        // The ink of 10 point digits whose baseline is at y 190
        assert!(two.min[0] >= 140.0 && two.min[0] < 141.5, "{two:?}");
        assert!(two.max[1] <= 190.5 && two.max[1] > 189.0, "{two:?}");
        assert!(two.min[1] > 181.0 && two.min[1] < 184.0, "{two:?}");
        let n = g.dimension_number([100.0, 200.0], [200.0, 200.0]).unwrap();
        assert_eq!(n.value, 2.0);
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
        // Each page's lines and words, and the numbers among them
        let page = std::env::var("BERC_PDF_PAGE")
            .ok()
            .and_then(|p| p.parse::<usize>().ok());
        for (i, g) in doc.geometry.iter().enumerate() {
            let Some(g) = g else { continue };
            if page.is_some_and(|p| p != i) {
                continue;
            }
            let numbers: Vec<&str> = g
                .words
                .iter()
                .filter(|w| super::super::geometry::length_number(&w.text).is_some())
                .map(|w| w.text.as_str())
                .take(40)
                .collect();
            eprintln!(
                "page {i}: {} lines, {} words, numbers {numbers:?}",
                g.segs.len(),
                g.words.len()
            );
        }
    }
}
