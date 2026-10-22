//! A synthetic Altium Smart PDF, written by our own code so the public repo carries no vendor document: the shape
//! of the real ones (bookmarks per sheet with Components, Nets and Ports, pin bookmarks with `_FindWord` scripts,
//! property popups through document-level `ShowCompProps_*` functions, invisible `CO`/`PI`/`NL` words), with the
//! cases the reader must handle: a sheet inside a sheet, a part whose units sit on two sheets, a bus with member
//! nets, a port no other sheet names, a popup whose `CO` word lies outside it (bound by its pins), words inside a
//! form XObject, a popup with no words, a cover page of another document with its own popup, a UTF-16 title, a
//! script with escapes and an inline popup.
//!
//! `tests/fixtures/smart_pdf.pdf` is this file, written by `BLESS=1 cargo test -p erc_import --test pdf`.

use pdf_writer::{Content, Name, Pdf, Rect, Ref, Str, TextStr};

enum Act {
    None,
    Page(usize),
    /// A page and a `_FindWord` script chained after the `GoTo`
    Find(usize, String),
}

struct Node {
    title: Vec<u8>,
    act: Act,
    kids: Vec<Node>,
}

fn node(title: &str, act: Act, kids: Vec<Node>) -> Node {
    Node {
        title: if title.is_ascii() {
            title.as_bytes().to_vec()
        } else {
            utf16(title)
        },
        act,
        kids,
    }
}

/// UTF-16BE with its byte order mark, as Altium writes titles that are not ASCII
fn utf16(title: &str) -> Vec<u8> {
    let mut b = vec![0xFE, 0xFF];
    for u in title.encode_utf16() {
        b.extend_from_slice(&u.to_be_bytes());
    }
    b
}

/// A component bookmark with its pin bookmarks
fn comp(page: usize, title: &str, pins: &[&str]) -> Node {
    node(
        title,
        Act::Page(page),
        pins.iter()
            .map(|p| node(p, Act::Find(page, format!("PI{}", ident(p))), vec![]))
            .collect(),
    )
}

/// A net bookmark: its pins, labels and ports
fn net(page: usize, name: Node, pins: &[&str], labels: &[&str], ports: &[&str]) -> Node {
    let mut n = name;
    for (section, list, prefix) in [("Pins", pins, "PI"), ("NetLabels", labels, "NL"), ("Ports", ports, "PO")] {
        if list.is_empty() {
            continue;
        }
        n.kids.push(node(
            section,
            Act::None,
            list.iter()
                .map(|t| node(t, Act::Find(page, format!("{prefix}{}", ident(t))), vec![]))
                .collect(),
        ));
    }
    n
}

/// A net that a port names on another sheet: Altium titles the port bookmark with the net's name on this sheet,
/// its search word carries the port's own name
fn port_net(page: usize, name: &str, pins: &[&str], port_name: &str) -> Node {
    let mut n = net(page, node(name, Act::Page(page), vec![]), pins, &[], &[]);
    n.kids.push(node(
        "Ports",
        Act::None,
        vec![node(name, Act::Find(page, format!("PO{}", ident(port_name))), vec![])],
    ));
    n
}

/// An entry of a sheet's Ports section
fn port(page: usize, name: &str) -> Node {
    node(name, Act::Find(page, format!("PO{}", ident(name))), vec![])
}

fn ident(t: &str) -> String {
    t.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '0' })
        .collect()
}

fn outline() -> Vec<Node> {
    let top = 1;
    let mcu = 2;
    let io = 3;
    vec![
        node("Cover.pdf", Act::Page(0), vec![]),
        node(
            "Schematic Prints(\"All Documents\",Physical)",
            Act::Page(top),
            vec![node(
                "Top.SchDoc(U_Top)",
                Act::Page(top),
                vec![
                    node(
                        "Components",
                        Act::None,
                        vec![
                            comp(top, "J1", &["J1-1", "J1-2", "J1-3", "J1-4", "J1-5"]),
                            comp(top, "R1", &["R1-1", "R1-2"]),
                            comp(top, "U1B", &["U1-5", "U1-6"]),
                        ],
                    ),
                    node(
                        "Nets",
                        Act::None,
                        vec![
                            net(top, node("EN_µC", Act::Page(top), vec![]), &["J1-4"], &["EN_µC"], &[]),
                            net(top, node("GND", Act::Page(top), vec![]), &["J1-3", "U1-6"], &[], &[]),
                            net(top, node("NetR1_2", Act::Page(top), vec![]), &["R1-2", "U1-5"], &[], &[]),
                            net(top, node("NRST", Act::Page(top), vec![]), &["J1-5"], &["NRST"], &[]),
                            net(top, node("SDA", Act::Page(top), vec![]), &["J1-2"], &[], &["SDA"]),
                            net(top, node("VIN", Act::Page(top), vec![]), &["J1-1", "R1-1"], &["VIN"], &[]),
                        ],
                    ),
                    node(
                        "MCU.SchDoc(U_MCU)",
                        Act::Page(mcu),
                        vec![
                            node(
                                "Components",
                                Act::None,
                                vec![
                                    comp(mcu, "C1", &["C1-1", "C1-2"]),
                                    comp(mcu, "R2", &["R2-1", "R2-2"]),
                                    comp(mcu, "R4", &["R4-1", "R4-2"]),
                                    comp(mcu, "U1A", &["U1-1", "U1-2", "U1-3", "U1-4", "U1-7"]),
                                ],
                            ),
                            node(
                                "Nets",
                                Act::None,
                                vec![
                                    node(
                                        "D[0..1]",
                                        Act::Page(mcu),
                                        vec![
                                            net(mcu, node("D0", Act::Page(mcu), vec![]), &["U1-2"], &["D0"], &[]),
                                            net(
                                                mcu,
                                                node("D1", Act::Page(mcu), vec![]),
                                                &["R2-2", "U1-3"],
                                                &["D1", "D1"],
                                                &[],
                                            ),
                                        ],
                                    ),
                                    net(mcu, node("EN", Act::Page(mcu), vec![]), &["R4-1"], &["EN"], &[]),
                                    net(
                                        mcu,
                                        node("GND", Act::Page(mcu), vec![]),
                                        &["C1-2", "R4-2", "U1-4"],
                                        &[],
                                        &[],
                                    ),
                                    port_net(mcu, "NetU1_7", &["U1-7"], "NRST"),
                                    net(
                                        mcu,
                                        node("SDA", Act::Page(mcu), vec![]),
                                        &["R2-1", "U1-1"],
                                        &["SDA"],
                                        &["SDA"],
                                    ),
                                    net(mcu, node("VDD_3V3", Act::Page(mcu), vec![]), &["C1-1"], &[], &["VDD_3V3"]),
                                ],
                            ),
                            node(
                                "Ports",
                                Act::None,
                                vec![
                                    port(mcu, "NRST"),
                                    port(mcu, "SDA"),
                                    port(mcu, "VDD_3V3"),
                                ],
                            ),
                        ],
                    ),
                    // A sibling of the MCU sheet: its NRST port meets the MCU's, its EN label does not
                    node(
                        "IO.SchDoc(U_IO)",
                        Act::Page(io),
                        vec![
                            node("Components", Act::None, vec![comp(io, "R3", &["R3-1", "R3-2"])]),
                            node(
                                "Nets",
                                Act::None,
                                vec![
                                    net(io, node("EN", Act::Page(io), vec![]), &["R3-2"], &["EN"], &[]),
                                    port_net(io, "NetR3_1", &["R3-1"], "NRST"),
                                ],
                            ),
                            node("Ports", Act::None, vec![port(io, "NRST")]),
                        ],
                    ),
                ],
            )],
        ),
    ]
}

/// Invisible words on a page (`3 Tr`), each at its own text matrix
fn words(c: &mut Content, list: &[(&[u8], f32, f32)]) {
    c.begin_text();
    c.set_font(Name(b"F1"), 4.0);
    c.set_text_rendering_mode(pdf_writer::types::TextRenderingMode::Invisible);
    for (w, x, y) in list {
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, *x, *y]);
        c.show(Str(w));
    }
    c.end_text();
}

const POPUPS: &[(&str, &str)] = &[
    (
        "ShowCompProps_C1",
        r#"function ShowCompProps_C1(){var sChoice = app.popUpMenu('Value: 100n','Comment: CAPACITOR','Library Reference: CAP','Supplier Part Number 1: 81-GRM1'); if (sChoice) {app.launchURL(sChoice, true);}}"#,
    ),
    (
        "ShowCompProps_J1",
        r#"function ShowCompProps_J1(){var sChoice = app.popUpMenu("Comment: Header","Footprint: PH_4","Library Reference: HEADER4","Value: ","Mfr. PN: 61300411121");}"#,
    ),
    (
        "ShowCompProps_R1",
        r#"function ShowCompProps_R1(){var sChoice = app.popUpMenu("Comment: RESISTOR","Value: 4k7","Description: Resistor \"thick film\"","Footprint: RESC0402M");}"#,
    ),
    (
        "ShowCompProps_U1",
        r#"function ShowCompProps_U1(){var sChoice = app.popUpMenu("Comment: STM32G0B1","Library Reference: STM32G0B1RE","Value: ");}"#,
    ),
    (
        "_FindWord",
        r#"function _FindWord(page, word){ this.pageNum = page; search.query(word); }"#,
    ),
];

/// The synthetic Smart PDF
pub fn synthetic() -> Vec<u8> {
    let mut pdf = Pdf::new();
    let mut next = 1;
    let mut id = || {
        let r = Ref::new(next);
        next += 1;
        r
    };
    let catalog = id();
    let pages = id();
    let font = id();
    let page_ids: Vec<Ref> = (0..4).map(|_| id()).collect();
    let content_ids: Vec<Ref> = (0..4).map(|_| id()).collect();
    let form = id();
    let outlines = id();
    let names = id();
    let js_root = id();
    let js_kids = [id(), id()];

    // Pages: a cover from another document, the top sheet, the MCU sheet
    let mut cover = Content::new();
    words(&mut cover, &[(b"COR1", 150.0, 150.0)]);
    let mut top = Content::new();
    top.begin_text();
    top.set_font(Name(b"F1"), 8.0);
    top.set_text_matrix([1.0, 0.0, 0.0, 1.0, 100.0, 390.0]);
    top.show(Str(b"J1"));
    top.end_text();
    words(
        &mut top,
        &[
            (b"COJ1", 100.5, 381.0),
            (b"PIJ101", 110.0, 370.0),
            (b"PIJ102", 110.0, 350.0),
            (b"PIJ103", 110.0, 330.0),
            (b"PIJ104", 110.0, 310.0),
            (b"COR1", 205.0, 341.0),
            (b"PIR101", 207.0, 338.0),
            (b"PIR102", 207.0, 302.0),
            // U1B's component word lies outside its popup: its pins name it
            (b"COU1B", 300.0, 346.0),
            (b"PIU105", 302.0, 320.0),
            (b"PIU106", 302.0, 300.0),
            (b"NLVIN", 150.0, 375.0),
            (b"NLEN0\xB5C", 150.0, 312.0),
            (b"PIJ105", 110.0, 300.0),
            (b"NLNRST", 150.0, 300.0),
        ],
    );
    let mut mcu = Content::new();
    words(
        &mut mcu,
        &[
            (b"COU1A", 100.0, 401.0),
            (b"PIU101", 110.0, 390.0),
            (b"PIU102", 110.0, 370.0),
            (b"PIU103", 110.0, 350.0),
            (b"PIU104", 110.0, 210.0),
            (b"COR2", 405.0, 341.0),
            (b"PIR201", 407.0, 338.0),
            (b"PIR202", 407.0, 302.0),
            (b"NLSDA", 250.0, 390.0),
            (b"NLD0", 250.0, 370.0),
            (b"NLD1", 250.0, 350.0),
            (b"NLEN", 250.0, 330.0),
            (b"PIU107", 110.0, 230.0),
            (b"COR4", 505.0, 341.0),
            (b"PIR401", 507.0, 338.0),
            (b"PIR402", 507.0, 302.0),
        ],
    );
    // C1's words are drawn by a form, moved into place by its matrix and the page's transform
    mcu.save_state();
    mcu.transform([1.0, 0.0, 0.0, 1.0, 100.0, 100.0]);
    mcu.x_object(Name(b"Fm0"));
    mcu.restore_state();
    let mut c1 = Content::new();
    words(
        &mut c1,
        &[(b"COC1", 5.0, 41.0), (b"PIC101", 7.0, 38.0), (b"PIC102", 7.0, 2.0)],
    );

    let mut io = Content::new();
    words(
        &mut io,
        &[
            (b"COR3", 205.0, 341.0),
            (b"PIR301", 207.0, 338.0),
            (b"PIR302", 207.0, 302.0),
            (b"NLEN", 250.0, 330.0),
        ],
    );
    for (i, c) in [cover, top, mcu, io].into_iter().enumerate() {
        let data = c.finish();
        pdf.stream(content_ids[i], &data);
    }
    let c1 = c1.finish();
    pdf.form_xobject(form, &c1)
        .bbox(Rect::new(0.0, 0.0, 50.0, 50.0))
        .matrix([1.0, 0.0, 0.0, 1.0, 200.0, 200.0]);
    pdf.type1_font(font).base_font(Name(b"Helvetica"));

    // Link annotations: the property popups and one link that is not a popup
    let mut annots: Vec<Vec<Ref>> = vec![Vec::new(); 4];
    let links: &[(usize, [f32; 4], &str)] = &[
        (0, [100.0, 100.0, 200.0, 200.0], "ShowCompProps_C1();"),
        (1, [100.0, 300.0, 130.0, 380.0], "ShowCompProps_J1();"),
        (1, [200.0, 300.0, 215.0, 340.0], "ShowCompProps_R1();"),
        (1, [300.0, 280.0, 360.0, 340.0], "ShowCompProps_U1();"),
        (1, [600.0, 100.0, 650.0, 150.0], "ShowCompProps_R1();"),
        (1, [700.0, 100.0, 750.0, 150.0], "this.print();"),
        (2, [100.0, 200.0, 200.0, 400.0], "ShowCompProps_U1();"),
        (2, [300.0, 300.0, 315.0, 340.0], "ShowCompProps_C1();"),
        (
            2,
            [400.0, 300.0, 415.0, 340.0],
            r#"var s = app.popUpMenu("Value: 10k", "Library Reference: RES", "Comment: RESISTOR");"#,
        ),
    ];
    for (page, r, js) in links {
        let a = id();
        pdf.indirect(a)
            .dict()
            .pair(Name(b"Type"), Name(b"Annot"))
            .pair(Name(b"Subtype"), Name(b"Link"))
            .pair(Name(b"Rect"), Rect::new(r[0], r[1], r[2], r[3]))
            .insert(Name(b"A"))
            .dict()
            .pair(Name(b"S"), Name(b"JavaScript"))
            .pair(Name(b"JS"), Str(js.as_bytes()));
        annots[*page].push(a);
    }

    for i in 0..4 {
        let mut p = pdf.page(page_ids[i]);
        p.parent(pages)
            .media_box(Rect::new(0.0, 0.0, 842.0, 595.0))
            .contents(content_ids[i]);
        p.insert(Name(b"Annots"))
            .array()
            .items(annots[i].iter().copied());
        let mut res = p.resources();
        res.fonts().pair(Name(b"F1"), font);
        if i == 2 {
            res.x_objects().pair(Name(b"Fm0"), form);
        }
    }
    pdf.pages(pages).kids(page_ids.iter().copied()).count(4);

    // Document scripts: a name tree of two leaves
    let mut script_refs = Vec::new();
    for (name, js) in POPUPS {
        let r = id();
        pdf.indirect(r)
            .dict()
            .pair(Name(b"S"), Name(b"JavaScript"))
            .pair(Name(b"JS"), Str(js.as_bytes()));
        script_refs.push((*name, r));
    }
    pdf.indirect(js_root)
        .dict()
        .insert(Name(b"Kids"))
        .array()
        .items(js_kids);
    for (k, chunk) in script_refs.chunks(3).enumerate() {
        let mut d = pdf.indirect(js_kids[k]).dict();
        let mut arr = d.insert(Name(b"Names")).array();
        for (name, r) in chunk {
            arr.item(Str(name.as_bytes()));
            arr.item(*r);
        }
    }
    pdf.indirect(names)
        .dict()
        .pair(Name(b"JavaScript"), js_root);

    // Bookmarks
    let tree = outline();
    let mut flat: Vec<(Ref, Option<Ref>, Option<Ref>, Option<Ref>, &Node, Vec<Ref>)> = Vec::new();
    fn assign<'n>(
        nodes: &'n [Node],
        parent: Ref,
        id: &mut dyn FnMut() -> Ref,
        out: &mut Vec<(Ref, Option<Ref>, Option<Ref>, Option<Ref>, &'n Node, Vec<Ref>)>,
    ) -> Vec<Ref> {
        let refs: Vec<Ref> = nodes.iter().map(|_| id()).collect();
        for (i, n) in nodes.iter().enumerate() {
            let kids = assign(&n.kids, refs[i], id, out);
            out.push((
                refs[i],
                Some(parent),
                i.checked_sub(1).map(|j| refs[j]),
                refs.get(i + 1).copied(),
                n,
                kids,
            ));
        }
        refs
    }
    let roots = assign(&tree, outlines, &mut id, &mut flat);
    for (r, parent, prev, next_ref, n, kids) in &flat {
        let mut d = pdf.indirect(*r).dict();
        d.pair(Name(b"Title"), Str(&n.title));
        if let Some(p) = parent {
            d.pair(Name(b"Parent"), *p);
        }
        if let Some(p) = prev {
            d.pair(Name(b"Prev"), *p);
        }
        if let Some(x) = next_ref {
            d.pair(Name(b"Next"), *x);
        }
        if let (Some(f), Some(l)) = (kids.first(), kids.last()) {
            d.pair(Name(b"First"), *f);
            d.pair(Name(b"Last"), *l);
            d.pair(Name(b"Count"), -(kids.len() as i32));
        }
        let page = match &n.act {
            Act::None => None,
            Act::Page(p) | Act::Find(p, _) => Some(*p),
        };
        if let Some(p) = page {
            let mut a = d.insert(Name(b"A")).dict();
            a.pair(Name(b"S"), Name(b"GoTo"));
            a.insert(Name(b"D"))
                .array()
                .item(page_ids[p])
                .item(Name(b"Fit"));
            if let Act::Find(p, w) = &n.act {
                a.insert(Name(b"Next"))
                    .dict()
                    .pair(Name(b"S"), Name(b"JavaScript"))
                    .pair(
                        Name(b"JS"),
                        Str(format!("_FindWord({p}, \"{w}\");").as_bytes()),
                    );
            }
        }
    }
    pdf.indirect(outlines)
        .dict()
        .pair(Name(b"Type"), Name(b"Outlines"))
        .pair(Name(b"First"), roots[0])
        .pair(Name(b"Last"), *roots.last().unwrap())
        .pair(Name(b"Count"), roots.len() as i32);

    pdf.catalog(catalog)
        .pages(pages)
        .outlines(outlines)
        .pair(Name(b"Names"), names);
    pdf.document_info(id())
        .creator(TextStr("Altium Designer (synthetic)"));
    pdf.finish()
}
