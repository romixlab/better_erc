//! A small PDF written by the tests themselves: pages of text and line drawings, no vendor material

/// A page: text lines (x, y from the top, size, text) and line segments (x1, y1, x2, y2), in points
#[derive(Default)]
pub struct TestPage {
    pub texts: Vec<(f32, f32, f32, String)>,
    pub lines: Vec<[f32; 4]>,
    /// Filled rectangles (x, y from the top, width, height)
    pub rects: Vec<[f32; 4]>,
    /// Line segments as `lines` with their width, points
    pub wide: Vec<([f32; 4], f32)>,
    /// Stroked paths of cubic Béziers: a start (x, y) and each curve's two control points and end (six numbers);
    /// closed when `true`
    pub curves: Vec<([f32; 2], Vec<[f32; 6]>, bool)>,
}

/// A circle as PDFs draw one, for [`TestPage::curves`]: four Béziers, centre and radius in points, y down
pub fn circle(c: [f32; 2], r: f32) -> ([f32; 2], Vec<[f32; 6]>, bool) {
    let k = 0.552_284_8 * r;
    let [x, y] = c;
    (
        [x + r, y],
        vec![
            [x + r, y - k, x + k, y - r, x, y - r],
            [x - k, y - r, x - r, y - k, x - r, y],
            [x - r, y + k, x - k, y + r, x, y + r],
            [x + k, y + r, x + r, y + k, x + r, y],
        ],
        true,
    )
}

/// The bytes of a PDF with `pages`, each `size` points (width, height), text in Helvetica
pub fn write(size: [f32; 2], pages: &[TestPage]) -> Vec<u8> {
    let [w, h] = size;
    // Objects: 1 catalog, 2 pages, 3 font, then a page and its content per page
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        String::new(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
    ];
    let mut kids = Vec::new();
    for p in pages {
        let mut c = String::new();
        for [x, y, rw, rh] in &p.rects {
            c += &format!("0.2 0.2 0.2 rg {x} {} {rw} {rh} re f\n", h - y - rh);
        }
        c += "0 0 0 RG 0.5 w\n";
        for [x1, y1, x2, y2] in &p.lines {
            c += &format!("{x1} {} m {x2} {} l S\n", h - y1, h - y2);
        }
        for ([x1, y1, x2, y2], width) in &p.wide {
            c += &format!("{width} w {x1} {} m {x2} {} l S\n", h - y1, h - y2);
        }
        for ([x0, y0], curves, closed) in &p.curves {
            c += &format!("0.5 w {x0} {} m\n", h - y0);
            for [x1, y1, x2, y2, x3, y3] in curves {
                c += &format!("{x1} {} {x2} {} {x3} {} c\n", h - y1, h - y2, h - y3);
            }
            c += if *closed { "h S\n" } else { "S\n" };
        }
        for (x, y, s, t) in &p.texts {
            let t = t
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            c += &format!("BT /F1 {s} Tf {x} {} Td ({t}) Tj ET\n", h - y);
        }
        let page_obj = objects.len() + 1;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Resources << /Font << /F1 3 0 R >> >> \
             /Contents {} 0 R >>",
            page_obj + 1
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{c}endstream", c.len()));
        kids.push(format!("{page_obj} 0 R"));
    }
    objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        pages.len()
    );
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}
