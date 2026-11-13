//! The review's side panel, built on vhrd_egui's property-panel widgets (`ve_theme::props`): a pane whose bar
//! keeps Accept and Next in sight while the sections below scroll, each section a sub-pane that folds and says on
//! its header what it holds (the symbol, the page, the scale, the placement), so a folded one hides nothing.
//!
//! The sections, top to bottom: Symbol and footprint, Datasheet and page, View, Ruler and scale, Measure,
//! 3D model, Placement, Review, Notes, SpaceMouse, Settings.

use super::*;
use crate::settings::{FootprintReviewSettings, Settings};
use better_erc::fp::record::LineStatus;
use egui::Response;
use recorded::{line_source, line_status};
use ve_theme::props::*;
use ve_theme::{Tone, UiExt as _};

/// Accept's button: the bar is narrow, so the button says the verb and its tooltip the rest
pub(super) const ACCEPT: &str = "Accept";

/// Next's button: the next symbol in the list that needs a review
pub(super) const NEXT: &str = "Next to review";

/// What a symbol's review status means, as a tone (the word says it, the tone backs it up)
fn status_tone(s: SymbolStatus) -> Tone {
    match s {
        SymbolStatus::Ok => Tone::Good,
        SymbolStatus::Changed | SymbolStatus::Malformed | SymbolStatus::Missing => Tone::Crit,
        SymbolStatus::Unreviewed | SymbolStatus::OtherVersion | SymbolStatus::Partial => Tone::Warn,
        SymbolStatus::Refused => Tone::Accent,
        SymbolStatus::NoFootprint => Tone::Off,
    }
}

fn line_tone(s: LineStatus) -> Tone {
    match s {
        LineStatus::Ok => Tone::Good,
        LineStatus::Changed | LineStatus::Malformed => Tone::Crit,
        LineStatus::OtherVersion => Tone::Warn,
    }
}

/// A button that is off with a reason: the reason on hover
fn button_if(ui: &mut Ui, text: impl Into<WidgetText>, tip: &str, why: Option<&str>) -> Response {
    let b = ui.add_enabled(why.is_none(), Button::new(text));
    match why {
        Some(why) => b.on_disabled_hover_text(why),
        None if tip.is_empty() => b,
        None => b.on_hover_text(tip),
    }
}

impl FootprintReview {
    /// The side panel: the bar, then the sections in a scroll area of their own
    pub(super) fn controls(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let keys = self.keys();
        // Accept and Next: worked out before the bar borrows its own locals
        let blocker = self.accept_blocker();
        let accept_tip = match &self.rt.target {
            Some(Target {
                symbol,
                from: SymbolsFrom::Project(_),
                ..
            }) => format!(
                "The footprint matches the drawing: writes the review hash into the {FIELD} field of {symbol}, the \
                 library symbol and the schematic's copies, after showing what it writes where; copies it too ({})",
                key_name(keys.accept)
            ),
            Some(t) => format!(
                "The footprint matches the drawing: writes the review hash into the {FIELD} field of library symbol \
                 {}, after showing what it writes where; copies it too ({})",
                t.symbol,
                key_name(keys.accept)
            ),
            None => String::new(),
        };
        let next = self.rt.symbols.as_ref().map(|l| {
            self.next_to_review()
                .map(|i| (i, l.rows[i].symbol.clone()))
                .ok_or("no other symbol in the list needs a review")
        });
        let (mut accept, mut go_next) = (false, false);
        Pane::new()
            .bar(|ui| {
                let r = ui
                    .add_enabled_ui(blocker.is_none(), |ui| {
                        ui.primary_button(ACCEPT, accept_tip.as_str())
                    })
                    .inner;
                let r = match &blocker {
                    Some(why) => r.on_disabled_hover_text(why),
                    None => r,
                };
                accept = r.clicked();
                let why = match &next {
                    None => {
                        Some("open a board or a symbol library: Next steps through its symbols")
                    }
                    Some(Err(why)) => Some(*why),
                    Some(Ok(_)) => None,
                };
                let tip = match &next {
                    Some(Ok((_, s))) => format!("{s} ({})", key_name(keys.next)),
                    _ => String::new(),
                };
                go_next = button_if(ui, NEXT, &tip, why).clicked();
            })
            .fill_height()
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("fp panel")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        self.source_section(ui, &ctx);
                        self.datasheet_section(ui, &ctx);
                        self.view_section(ui);
                        self.ruler_section(ui);
                        self.measure_section(ui);
                        self.model_section(ui);
                        self.model_views_section(ui);
                        self.placement_section(ui);
                        self.review_section(ui, &ctx);
                        self.notes_section(ui);
                        self.puck_controls(ui);
                        self.settings_section(ui);
                    });
            });
        if accept {
            self.accept(&ctx);
        }
        if go_next && let Some(Ok((i, _))) = next {
            self.select_symbol(i, true);
            self.usage("next");
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // Symbol and footprint

    fn source_section(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let summary = match (&self.rt.target, &self.rt.sourced) {
            (Some(t), _) => t.symbol.clone(),
            (None, Some(s)) => format!("{} (preview)", s.lib_id()),
            (None, None) => "nothing open".into(),
        };
        SubPane::new("Symbol and footprint")
            .id_salt("fp source")
            .summary(summary)
            .tooltip(
                "What is reviewed: a library symbol, which holds the review hash, and the footprint its Footprint \
                 field names",
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button("Open board or project…")
                        .on_hover_text(
                            "A KiCad project (.kicad_pro, .kicad_sch, .kicad_pcb): the library symbols its board uses",
                        )
                        .clicked()
                        && let Some(p) = rfd::FileDialog::new()
                            .add_filter("KiCad project", &["kicad_pro", "kicad_sch", "kicad_pcb"])
                            .pick_file()
                    {
                        self.open_path(ctx, &p);
                    }
                    if ui
                        .button("Open symbol library…")
                        .on_hover_text("A .kicad_sym file: its symbols")
                        .clicked()
                        && let Some(p) = rfd::FileDialog::new()
                            .add_filter("KiCad symbol library", &["kicad_sym"])
                            .pick_file()
                    {
                        self.open_path(ctx, &p);
                    }
                    if ui
                        .button("Open symbol folder…")
                        .on_hover_text("A .kicad_symdir directory (KiCad 9 and later): its symbols")
                        .clicked()
                        && let Some(p) = rfd::FileDialog::new().pick_folder()
                    {
                        self.open_path(ctx, &p);
                    }
                    if ui
                        .button("Open a footprint alone (preview)…")
                        .on_hover_text(
                            "A library footprint (.kicad_mod) to look at over a datasheet; Accept needs a symbol to \
                             write into",
                        )
                        .clicked()
                        && let Some(p) = rfd::FileDialog::new()
                            .add_filter("KiCad footprint", &["kicad_mod"])
                            .pick_file()
                    {
                        self.open_path(ctx, &p);
                    }
                });
                if let Some(e) = &self.rt.error {
                    notice(ui, Tone::Crit, e);
                }
                self.table_libraries(ui);
                self.symbol_list(ui);
                self.symbol_info(ui);
                self.board_list(ui);
                self.footprint_info(ui);
            });
    }

    /// The symbol libraries the tables name, to open one
    fn table_libraries(&mut self, ui: &mut Ui) {
        let dir = self
            .rt
            .symbols
            .as_ref()
            .map_or_else(|| PathBuf::from("."), |l| l.from.dir());
        let mut open = None;
        SubPane::new("Symbol libraries in the tables")
            .id_salt("table libraries")
            .summary("sym-lib-table")
            .tooltip("The symbol libraries the project's and KiCad's sym-lib-tables name: click one to open it")
            .default_open(false)
            .show(ui, |ui| {
                if self.rt.tables.as_ref().is_none_or(|t| t.0 != dir) {
                    let libs = SymbolLibraries::new(&dir, self.rt.config.as_deref());
                    let names = libs
                        .tables()
                        .libraries()
                        .iter()
                        .map(|e| e.name.clone())
                        .collect();
                    self.rt.tables = Some((dir.clone(), names));
                }
                let Some((_, names)) = &self.rt.tables else {
                    return;
                };
                if names.is_empty() {
                    hint(ui, "no sym-lib-table here or in KiCad's configuration");
                    return;
                }
                filter_field(ui, &mut self.rt.tables_filter, "filter: nickname");
                let f = self.rt.tables_filter.to_lowercase();
                ListView::new("table libraries list")
                    .max_height(160.0)
                    .show(ui, |list| {
                        for n in names {
                            if (f.is_empty() || n.to_lowercase().contains(&f))
                                && list.row(ListRow::new(n).tooltip("Open its symbols")).clicked()
                            {
                                open = Some(n.clone());
                            }
                        }
                        if list.is_empty() {
                            list.empty("no nickname has the filter's text");
                        }
                    });
            });
        if let Some(n) = open {
            self.open_symbols(SymbolsFrom::Library {
                library: n,
                project: dir,
            });
            self.usage("library");
        }
    }

    /// The symbols to review, each with what it needs
    fn symbol_list(&mut self, ui: &mut Ui) {
        let mut picked = None;
        let current = self.rt.target.as_ref().map(|t| t.symbol.clone());
        let Some(l) = &mut self.rt.symbols else {
            hint(
                ui,
                "Open a board or a symbol library: the review is of a library symbol, which holds the hash",
            );
            return;
        };
        let to_review = l
            .rows
            .iter()
            .filter(|r| !matches!(r.status, SymbolStatus::Ok | SymbolStatus::NoFootprint))
            .count();
        SubPane::new(l.title.clone())
            .id_salt("symbols")
            .summary(format!("{to_review} open"))
            .tooltip("The symbols, each with its review status; click one to review it")
            .show(ui, |ui| {
                filter_field(ui, &mut l.filter, "filter: symbol, footprint or reference");
                let f = l.filter.to_lowercase();
                ListView::new("symbol rows")
                    .max_height(260.0)
                    .show(ui, |list| {
                        for (i, r) in l.rows.iter().enumerate() {
                            let hit = |t: &str| t.to_lowercase().contains(&f);
                            if !f.is_empty()
                                && !hit(&r.symbol)
                                && !r.footprint.as_deref().is_some_and(hit)
                                && !r.references.iter().any(|x| hit(x))
                            {
                                continue;
                            }
                            let mut hover = String::new();
                            if let Some(fp) = &r.footprint {
                                hover.push_str(&format!("Footprint {fp}"));
                            }
                            if !r.references.is_empty() {
                                hover.push_str(&format!("\non {}", r.references.join(", ")));
                            }
                            if let Some(fp) = r.overridden_footprint() {
                                hover.push_str(&format!("\nplaced as {fp}, not its own Footprint"));
                            }
                            for lib in [
                                &r.symbol_library_disallowed,
                                &r.footprint_library_disallowed,
                            ]
                            .into_iter()
                            .flatten()
                            {
                                hover.push_str(&format!(
                                    "\nlibrary {lib} is not in the allowed libraries"
                                ));
                            }
                            if let Some(n) = &r.note {
                                hover.push_str(&format!("\n{n}"));
                            }
                            let row = ListRow::new(&r.symbol)
                                .lead(r.status.name(), status_tone(r.status))
                                .selected(current.as_deref() == Some(r.symbol.as_str()))
                                .tooltip(hover.trim_start().to_string());
                            if list.row(row).clicked() {
                                picked = Some(i);
                            }
                        }
                        if list.is_empty() {
                            list.empty("no symbol, footprint or reference has the filter's text");
                        }
                    });
            });
        if let Some(i) = picked {
            self.select_symbol(i, true);
        }
    }

    /// The symbol under review: its footprint and datasheet fields, or why it has no footprint to review
    fn symbol_info(&self, ui: &mut Ui) {
        let Some(t) = &self.rt.target else { return };
        let row = self
            .symbol_row(&t.symbol)
            .and_then(|i| self.rt.symbols.as_ref().map(|l| &l.rows[i]));
        Group::untitled().show(ui, |ui| {
            Facts::new("symbol").show(ui, |f| {
                f.fact("Symbol", &t.symbol, "");
                if let Some(r) = row {
                    f.fact_tone("Status", r.status.name(), status_tone(r.status), "");
                    f.fact(
                        "Footprint",
                        r.footprint.as_deref().unwrap_or("(none)"),
                        "its Footprint field",
                    );
                    if let Some(d) = &r.datasheet {
                        f.fact("Datasheet", d, "its Datasheet field");
                    }
                    if !r.references.is_empty() {
                        f.fact("On the board", r.references.join(", "), "");
                    }
                    if let Some(fp) = r.overridden_footprint() {
                        f.fact(
                            "Placed",
                            fp,
                            "the board places this instead of the symbol's own Footprint above",
                        );
                    }
                }
            });
            if let Some(r) = row {
                for lib in [
                    &r.symbol_library_disallowed,
                    &r.footprint_library_disallowed,
                ]
                .into_iter()
                .flatten()
                {
                    notice(
                        ui,
                        Tone::Warn,
                        format!("library {lib} is not in the allowed libraries"),
                    );
                }
            }
            if let Some(n) = &self.rt.symbol_note {
                notice(ui, Tone::Warn, n);
            }
        });
    }

    fn board_list(&mut self, ui: &mut Ui) {
        let mut picked = None;
        let current = self.rt.sourced.as_ref().map(|s| s.lib_id().to_string());
        let Some(b) = &mut self.rt.board else { return };
        let title = format!(
            "{} library footprints on {} (preview)",
            b.groups.len(),
            b.path
                .file_name()
                .map(|f| f.to_string_lossy())
                .unwrap_or_default()
        );
        SubPane::new(title)
            .id_salt("board footprints")
            .tooltip("The board's footprints by library id: one opened alone, as a preview over its datasheet")
            .default_open(false)
            .show(ui, |ui| {
                filter_field(ui, &mut b.filter, "filter: library id or reference");
                let f = b.filter.to_lowercase();
                ListView::new("board footprints list").max_height(220.0).show(ui, |list| {
                    for (lib_id, refs, i) in &b.groups {
                        if !f.is_empty()
                            && !lib_id.to_lowercase().contains(&f)
                            && !refs.iter().any(|r| r.to_lowercase().contains(&f))
                        {
                            continue;
                        }
                        let row = ListRow::new(format!("{lib_id}  ({})", refs.join(", ")))
                            .selected(current.as_deref() == Some(lib_id.as_str()))
                            .tooltip(refs.join(", "));
                        if list.row(row).clicked() {
                            picked = Some(*i);
                        }
                    }
                    if list.is_empty() {
                        list.empty("no library id or reference has the filter's text");
                    }
                });
            });
        if let Some(i) = picked {
            self.pick(i);
        }
    }

    /// The footprint shown: its library id, where it comes from, its review hash and the one recorded
    fn footprint_info(&self, ui: &mut Ui) {
        let Some(s) = &self.rt.sourced else { return };
        Group::untitled().show(ui, |ui| {
            if self.rt.target.is_none() {
                notice(
                    ui,
                    Tone::Warn,
                    "Preview: no symbol to write the review hash into",
                );
            }
            Facts::new("fp").show(ui, |f| {
                f.fact("Footprint", s.lib_id(), "");
                f.fact("From", short_source(&s.source), s.source.describe());
                f.fact_mono(
                    "Review hash",
                    &s.hash,
                    "what Accept writes: a hash of the footprint's pads and outline",
                );
                // Every review line's hash against this footprint's: the first that differs, else the one they share
                let rec = self.recorded();
                if let Some(stored) = rec.stored() {
                    if rec.stale().is_empty() {
                        f.fact_tone(
                            FIELD,
                            format!("{stored}, the same: reviewed"),
                            Tone::Good,
                            "the symbol's review hash",
                        );
                    } else {
                        f.fact_tone(
                            FIELD,
                            format!("{stored}: not this footprint, review it again"),
                            Tone::Warn,
                            "the symbol's review hash",
                        );
                    }
                }
            });
            if let Some(n) = &self.rt.board_note {
                hint(ui, n);
            }
        });
    }

    // -----------------------------------------------------------------------------------------------------
    // Datasheet and page

    fn datasheet_section(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let summary = match &self.rt.pdf {
            Some(pdf) => format!(
                "{} · p.{} of {}",
                file_name(&pdf.path),
                self.page + 1,
                pdf.page_count()
            ),
            None => "no datasheet open".into(),
        };
        SubPane::new("Datasheet and page")
            .id_salt("fp datasheet")
            .summary(summary)
            .tooltip("The datasheet the footprint is compared with, and its land pattern page")
            .show(ui, |ui| self.datasheet_controls(ui, ctx));
    }

    fn datasheet_controls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let field = self
            .rt
            .sourced
            .as_ref()
            .and_then(|s| s.field("Datasheet"))
            .map(String::from);
        let local = self.datasheet_file();
        ui.horizontal_wrapped(|ui| {
            if let (Some(f), Some(p)) = (&field, &local)
                && ui
                    .button("Open the Datasheet field's file")
                    .on_hover_text(f)
                    .clicked()
            {
                let p = p.clone();
                self.open_datasheet(ctx, &p);
            }
            if ui
                .button("Open PDF…")
                .on_hover_text("A datasheet PDF from a file picker")
                .clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf", "PDF"])
                    .pick_file()
            {
                self.open_datasheet(ctx, &p);
            }
        });
        match (&field, &local) {
            (Some(_), Some(_)) => {}
            (Some(f), None) if f.starts_with("http://") || f.starts_with("https://") => {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Datasheet field:");
                    ui.hyperlink_to(f, f);
                });
                hint(
                    ui,
                    "A web address: find a local copy below, or save it and open it",
                );
            }
            (Some(f), None) => {
                hint(ui, format!("Datasheet field: {f} (no such file)"));
            }
            (None, _) if self.rt.sourced.is_some() => {
                hint(ui, "The footprint has no Datasheet field");
            }
            _ => {}
        }
        self.mpn_table(ui, ctx);
        // Library search
        let cmd = self.rt.settings.library_search.clone();
        let blocker = if self.rt.searcher.is_some() {
            None
        } else if cmd.is_empty() {
            Some("no library search command is set (Settings)".to_string())
        } else if find_program(&cmd[0]).is_none() {
            Some(format!(
                "{} is not on PATH (Settings: library search)",
                cmd[0]
            ))
        } else if self.rt.query.trim().is_empty() {
            Some("type the words to search for".into())
        } else if self.rt.searching.is_some() {
            Some("searching…".into())
        } else {
            None
        };
        PropertyRow::new("Library search")
            .tooltip("The document library searched for the part number (Settings: library search)")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 120.0).max(80.0);
                    ui.add(
                        TextEdit::singleline(&mut self.rt.query)
                            .hint_text("part number")
                            .desired_width(w),
                    );
                    if button_if(ui, "Search library", &cmd.join(" "), blocker.as_deref()).clicked()
                    {
                        self.library_search(ctx, false);
                    }
                });
            });
        if let Some(why) = &blocker
            && self.rt.searching.is_none()
        {
            hint(ui, why);
        }
        let mut open = None;
        match &self.rt.search_result {
            Some(Ok(hits)) if hits.is_empty() => {
                hint(ui, "The library has nothing for that");
            }
            Some(Ok(hits)) => {
                // A group of its own, so the found datasheets don't hide among the other controls
                Group::new(format!(
                    "Found in the library ({}), the first opens by itself:",
                    hits.len()
                ))
                .show(ui, |ui| {
                    ListView::new("library hits")
                        .max_height(160.0)
                        .show(ui, |list| {
                            for h in hits {
                                let is_open = self.datasheet.as_ref() == Some(&h.path);
                                let name = file_name(&h.path);
                                let dir = h
                                    .path
                                    .parent()
                                    .map(|d| d.display().to_string())
                                    .unwrap_or_default();
                                let text =
                                    format!("📄 {name}{}", if is_open { "  (open)" } else { "" });
                                if list
                                    .row(
                                        ListRow::new(text)
                                            .selected(is_open)
                                            .tooltip(format!("Open it\n{dir}")),
                                    )
                                    .clicked()
                                {
                                    open = Some(h.clone());
                                }
                            }
                        });
                });
            }
            Some(Err(e)) => {
                notice(ui, Tone::Crit, e);
            }
            None => {}
        }
        if let Some(h) = open {
            self.open_hit(ctx, &h);
        }
        // The open datasheet and its pages
        let Some(pdf) = &self.rt.pdf else { return };
        if let Some(e) = &pdf.error {
            notice(ui, Tone::Crit, format!("{}: {e}", file_name(&pdf.path)));
            return;
        }
        let pages = pdf.page_count();
        let mut page = self.page;
        Group::new(file_name(&pdf.path))
            .tooltip(pdf.path.display().to_string())
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if button_if(
                        ui,
                        egui_phosphor::regular::CARET_LEFT,
                        "the page before",
                        (page == 0).then_some("the first page"),
                    )
                    .clicked()
                    {
                        page -= 1;
                    }
                    ui.label(format!("page {} of {pages}", page + 1));
                    if button_if(
                        ui,
                        egui_phosphor::regular::CARET_RIGHT,
                        "the page after",
                        (page + 1 >= pages).then_some("the last page"),
                    )
                    .clicked()
                    {
                        page += 1;
                    }
                    if ui
                        .button("Full width")
                        .on_hover_text(format!(
                            "The page at the view's full width; {}",
                            key_name(KeyMap::from(&self.rt.settings.keys).full_width)
                        ))
                        .clicked()
                    {
                        self.rt.fit_width = true;
                    }
                    if ui
                        .button("Whole page")
                        .on_hover_text("The whole page in the view")
                        .clicked()
                    {
                        self.rt.fit_page = true;
                    }
                });
                let words = &self.rt.settings.page_search_words;
                match &self.rt.page_hits {
                    None => {
                        hint(ui, "finding the land pattern page…");
                    }
                    Some(hits) if hits.is_empty() => {
                        hint(ui, "no page has the search words (Settings)");
                    }
                    Some(hits) => {
                        PropertyRow::new("Pages with the search words")
                            .tooltip(
                                "The page search's hits, best first (Settings: page search words)",
                            )
                            .show(ui, |ui| {
                                let mut seg = Segmented::new(&mut page).id_salt("page hits");
                                for h in hits {
                                    let w = h
                                        .words
                                        .iter()
                                        .filter_map(|i| words.get(*i).map(String::as_str))
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    seg = seg.item(h.page, format!("p.{}", h.page + 1), w);
                                }
                                if seg.show(ui).changed() {
                                    self.usage("page-search");
                                }
                            });
                    }
                }
            });
        if page != self.page {
            self.page = page;
            self.rt.fit_page = true;
        }
    }

    /// The symbol's datasheets by MPN (VIEW-26): each MPN, the library document that covers it (a click opens it,
    /// with its review line's view, else the page search) and its review state; an MPN with no document says why,
    /// with a button that fetches it
    fn mpn_table(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if self.symbol_mpns().is_empty() {
            return;
        }
        if let Some(why) = self.mpn_lookup_blocker() {
            hint(ui, format!("MPN datasheets: {why}"));
            return;
        }
        let rows = self.mpn_rows();
        let fetch_blocker = self.mpn_fetch_blocker();
        let lookup_failed = self.mpn_lookup_error();
        let t = ve_theme::Tokens::of_ui(ui);
        let fill = ui.visuals().panel_fill;
        let mut open = None;
        let mut fetch = None;
        Group::new("Datasheets by MPN")
            .tooltip(
                "Each MPN of the symbol, the library document that covers it and its review state (Settings: \
                 library MPN lookup)",
            )
            .show(ui, |ui| {
                if let Some(e) = &lookup_failed {
                    notice(ui, Tone::Crit, format!("the library's MPN lookup failed: {e}"));
                }
                egui::Grid::new("mpn datasheets")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        for r in &rows {
                            ui.label(&r.mpn);
                            match (&r.doc, &r.missing) {
                                (Some(d), _) => {
                                    let path = d.path.clone().unwrap_or_default();
                                    let is_open = self.datasheet.as_ref() == Some(&path);
                                    let mut tip = path.display().to_string();
                                    if let Some(no) = d.label() {
                                        tip = format!("{no}\n{tip}");
                                    }
                                    let text = format!("📄 {}", file_name(&path));
                                    if ui
                                        .selectable_label(is_open, text)
                                        .on_hover_text(format!("Open it\n{tip}"))
                                        .clicked()
                                    {
                                        open = Some(path);
                                    }
                                }
                                (None, Some(why)) => {
                                    ui.horizontal(|ui| {
                                        let fetching = self
                                            .rt
                                            .mpn_docs
                                            .fetching
                                            .as_ref()
                                            .is_some_and(|(m, _)| *m == r.mpn);
                                        let label = if fetching { "fetching…" } else { "Fetch" };
                                        if button_if(
                                            ui,
                                            label,
                                            &format!("Fetch {}'s datasheet into the library", r.mpn),
                                            fetch_blocker.as_deref(),
                                        )
                                        .clicked()
                                        {
                                            fetch = Some(r.mpn.clone());
                                        }
                                        ui.weak("no document").on_hover_text(why);
                                    });
                                }
                                (None, None) => {
                                    ui.weak("asking the library…");
                                }
                            }
                            let (word, tone) = r.review.word();
                            let ink = t.legible(tone.ink(&t), fill);
                            ui.colored_label(ink, word).on_hover_text(r.review.detail());
                            ui.end_row();
                        }
                    });
                if let Some((m, r)) = &self.rt.mpn_docs.fetched {
                    match r {
                        Ok(said) => {
                            hint(ui, format!("fetched {m}: {said}"));
                        }
                        Err(e) => {
                            notice(ui, Tone::Crit, format!("fetching {m}: {e}"));
                        }
                    }
                }
            });
        if let Some(p) = open {
            self.open_datasheet(ctx, &p);
            self.usage("mpn datasheet");
        }
        if let Some(m) = fetch {
            self.fetch_mpn(ctx, &m);
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // View

    fn view_section(&mut self, ui: &mut Ui) {
        let opacity = self.opacity.unwrap_or(self.rt.settings.opacity);
        let summary = format!(
            "{} · {} · {:.0} %",
            self.cu_shown.name(),
            self.blend_mode().name(),
            opacity * 100.0
        );
        SubPane::new("View")
            .id_salt("fp view")
            .summary(summary)
            .tooltip(
                "How the footprint is drawn over the page: which layer, how it blends, how opaque",
            )
            .show(ui, |ui| self.overlay_controls(ui));
    }

    fn overlay_controls(&mut self, ui: &mut Ui) {
        let k = self.keys();
        Group::new("Layer")
            .tooltip("What the board makes on copper, mask or paste: one view at a time, each in its colour (Settings)")
            .show(ui, |ui| {
                let mut shown = self.cu_shown;
                let mut seg = Segmented::new(&mut shown).id_salt("cu views").fill_width();
                for v in VIEWS {
                    seg = seg.item(v, v.name(), format!("The footprint's {} ({} cycles)", v.name().to_lowercase(), key_name(k.cu_layer)));
                }
                seg.show(ui);
                self.cu_shown = shown;
                ui.horizontal_wrapped(|ui| {
                    for name in std::iter::once(BACK).chain(Kind::OWN.map(Kind::name)) {
                        let mut on = self.group_shown(name);
                        if ui
                            .add(ToggleChip::new(&mut on, name).tooltip(format!("Show the footprint's {}", name.to_lowercase())))
                            .changed()
                        {
                            self.hidden.retain(|h| h != name);
                            if !on {
                                self.hidden.push(name.to_string());
                            }
                        }
                    }
                });
            });
        // Blend mode: the modes other than normal need the GPU pass
        let gpu = blend::available(ui.ctx());
        let mut mode = self.blend_mode();
        Group::new("Blend")
            .tooltip("How the footprint's colour mixes with the page under it")
            .show(ui, |ui| {
                let mut seg = Segmented::new(&mut mode).id_salt("blend").fill_width();
                for m in BlendMode::ALL {
                    let why = (m != BlendMode::Normal && !gpu).then_some(
                        "needs the app's wgpu renderer, which this window does not have",
                    );
                    seg = seg.item_if(m, m.name(), blend_hint(m), why);
                }
                seg.show(ui);
            });
        if mode != self.blend_mode() {
            self.blend = Some(mode);
            self.usage(&format!("blend {}", mode.name().to_lowercase()));
        }
        Group::new("Page and overlay").show(ui, |ui| {
            let mut opacity = self.opacity.unwrap_or(self.rt.settings.opacity);
            PropertyRow::new("Opacity")
                .tooltip("0: the page alone, 1: the footprint hides the page under it")
                .show(ui, |ui| {
                    if ui
                        .add(
                            ScalarField::new(&mut opacity)
                                .range(0.0..=1.0)
                                .speed(0.005)
                                .decimals(2)
                                .tooltip("How much of the page the footprint covers"),
                        )
                        .changed()
                    {
                        self.opacity = Some(opacity);
                    }
                });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(ToggleChip::new(&mut self.blink, "Blink").tooltip(format!(
                        "The footprint on and off, {} times a second (Settings); {}",
                        self.rt.settings.blink_hz,
                        key_name(k.blink)
                    )))
                    .changed()
                    && self.blink
                {
                    self.usage("blink");
                }
                let mut invert = self.page_inverted();
                if ui
                    .add(
                        ToggleChip::new(&mut invert, "Invert page")
                            .tooltip(format!("White-ish on black; {}", key_name(k.invert))),
                    )
                    .changed()
                {
                    self.invert = Some(invert);
                    self.usage("invert");
                }
            });
        });
    }

    // -----------------------------------------------------------------------------------------------------
    // Ruler and scale

    fn ruler_section(&mut self, ui: &mut Ui) {
        let summary = match &self.scale {
            Some(s) => format!("{:.4} mm/pt, p.{}", s.mm_per_point, s.page + 1),
            None => "not set (1:1)".into(),
        };
        SubPane::new("Ruler and scale")
            .id_salt("fp scale")
            .summary(summary)
            .tooltip("The drawing's scale, set with the ruler along a dimension whose length the drawing gives")
            .show(ui, |ui| self.scale_controls(ui));
    }

    fn scale_controls(&mut self, ui: &mut Ui) {
        let k = self.keys();
        let why = self
            .ruler
            .is_none()
            .then(|| format!("draw the ruler first ({})", key_name(k.ruler)));
        if button_if(
            ui,
            "Zoom on an end",
            &format!(
                "In on the first end to place it exactly, again on the other, again back ({})",
                key_name(k.ruler_zoom)
            ),
            why.as_deref(),
        )
        .clicked()
        {
            let c = self.rt.canvas;
            self.ruler_zoom_key(c);
        }
        if let Some(h) = self.ruler_hint() {
            notice(ui, Tone::Accent, h);
        }
        match &self.scale {
            Some(s) => {
                Facts::new("scale").show(ui, |f| {
                    f.fact(
                        "Scale",
                        format!(
                            "{:.4} mm per point ({:.2}:1)",
                            s.mm_per_point,
                            MM_PER_POINT / s.mm_per_point
                        ),
                        "",
                    );
                    f.fact("Set on", format!("page {}", s.page + 1), "");
                    if let Some(n) = &s.note {
                        f.fact("From", format!("the page's \"{n}\""), "");
                    }
                });
            }
            None => {
                hint(
                    ui,
                    "Not set: the PDF's own scale (1:1), which drawings rarely are",
                );
            }
        }
        // The page's own scale note, when it has one
        let note = self
            .rt
            .pdf
            .as_ref()
            .and_then(|p| p.texts.get(self.page)?.as_deref())
            .and_then(scale_note);
        if let Some((text, factor)) = note
            && ui
                .button(format!("Use the page's \"{text}\""))
                .on_hover_text("The drawing says its scale; check it with the ruler on a dimension")
                .clicked()
        {
            self.scale = Some(Scale {
                mm_per_point: MM_PER_POINT / factor,
                page: self.page,
                mm: 0.0,
                note: Some(text),
            });
            self.scale_set();
            self.usage("scale page-note");
        }
        if self.tool.measures() {
            hint(
                ui,
                format!(
                    "Measuring: the scale's ruler waits in the Scale mode ({})",
                    key_name(k.ruler)
                ),
            );
            return;
        }
        let Some([a, b]) = self.ruler else {
            hint(
                ui,
                "Ruler: drag along a dimension whose length the drawing gives",
            );
            return;
        };
        let pts = ((b[0] - a[0]) as f64).hypot((b[1] - a[1]) as f64);
        Facts::new("ruler").show(ui, |f| {
            f.fact(
                "Ruler",
                format!("{:.3} mm at this scale", pts * self.mm_per_point()),
                "the ruler's length at the scale in use",
            );
        });
        let mut set = false;
        PropertyRow::new("The dimension is")
            .tooltip(format!(
                "The length the drawing gives for the dimension under the ruler; Enter or Set scale ({} jumps here)",
                key_name(k.scale)
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let r = ui.add(
                        TextEdit::singleline(&mut self.rt.ruler_mm)
                            .desired_width(80.0)
                            .id_salt("ruler mm"),
                    );
                    ui.label("mm");
                    self.rt.typing_mm = r.has_focus() || self.rt.length_box;
                    set = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let ok = self
                        .rt
                        .ruler_mm
                        .trim()
                        .replace(',', ".")
                        .parse::<f64>()
                        .is_ok_and(|v| v > 0.0)
                        && pts > 0.0;
                    set |= button_if(
                        ui,
                        "Set scale",
                        "The ruler's length is this many mm: the drawing's scale",
                        (!ok).then_some("type the dimension's length in mm"),
                    )
                    .clicked();
                });
            });
        if set {
            self.set_scale_from_ruler();
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // Measure

    fn measure_section(&mut self, ui: &mut Ui) {
        let summary = match &self.rt.measured {
            Some(m) if !m.text.is_empty() => m.text.clone(),
            _ if self.tool.measures() => self.tool.name().to_string(),
            _ => "no measurement".into(),
        };
        SubPane::new("Measure")
            .id_salt("fp measure")
            .summary(summary)
            .tooltip("A ruler on the PDF or on the footprint: its button adds its length to the review note")
            .show(ui, |ui| {
                let k = self.keys();
                let pdf = self.rt.pdf.as_ref().is_some_and(|p| p.page_count() > 0);
                let fp = !self.rt.items.is_empty();
                let mut pick = None;
                ui.horizontal_wrapped(|ui| {
                    for (t, text, key, why) in [
                        (
                            Tool::MeasurePdf,
                            "Measure the PDF",
                            k.measure_pdf,
                            (!pdf).then_some("open the datasheet first: the ruler lies on its drawing"),
                        ),
                        (
                            Tool::MeasureFootprint,
                            "Measure the footprint",
                            k.measure_footprint,
                            (!fp).then_some("no footprint shown: pick a symbol"),
                        ),
                        (
                            Tool::MeasureAcross,
                            "Measure across",
                            k.measure_across,
                            self.across_blocker(),
                        ),
                    ] {
                        let on = self.tool == t;
                        let r = ui.add_enabled(why.is_none() || on, Button::selectable(on, text));
                        let r = match why {
                            Some(why) => r.on_disabled_hover_text(why),
                            None => r.on_hover_text(format!(
                                "{}; {} (again: back to Move)",
                                t.what(),
                                key_name(key)
                            )),
                        };
                        if r.clicked() {
                            pick = Some(t);
                        }
                    }
                });
                if let Some(t) = pick {
                    self.tool_key(t);
                }
                if self.tool.measures()
                    && let Some(ends) = self.ruler
                {
                    let (mm, dx, dy) = self.ruler_mm(ends);
                    Facts::new("measure").show(ui, |f| {
                        f.fact("Length", format!("{mm:.3} mm"), "the ruler's length at the drawing's scale");
                        f.fact("dX, dY", format!("{dx:.3}, {dy:.3} mm"), "across and down the page");
                    });
                }
                // The ruler's button, as beside its label on the canvas
                let why = self.measurement_blocker();
                let text = match self.noted() {
                    Some(m) => format!("In the note as M{}", m.n),
                    None => format!("Add to the note as M{}", self.rt.measure_count + 1),
                };
                let r = ui.add_enabled(why.is_none(), Button::new(text));
                let r = match &why {
                    Some(w) => r.on_disabled_hover_text(w),
                    None => r.on_hover_text("the ruler's length into the review note"),
                };
                if r.clicked() {
                    self.add_measurement();
                }
                self.model_measure_row(ui);
                if let Some(m) = &self.rt.measured
                    && !m.text.is_empty()
                {
                    Facts::new("measured").show(ui, |f| {
                        f.fact("Last added", &m.text, "the last ruler added to the review note");
                    });
                }
                hint(
                    ui,
                    format!(
                        "A measurement goes into the review note only by its button (+ note beside its label), as \
                         M1, M2, …; {} or Esc: back to Move",
                        key_name(k.move_tool)
                    ),
                );
            });
    }

    // -----------------------------------------------------------------------------------------------------
    // Placement

    fn placement_section(&mut self, ui: &mut Ui) {
        let k = self.mm_per_point();
        let place = self.moving_mut().clone();
        let fp = self.model_views.moves == modelviews::Moves::Footprint;
        let summary = format!(
            "{}{:.2}, {:.2} mm · {:.0}° · {}",
            if fp {
                String::new()
            } else {
                format!("{} · ", self.moving_name())
            },
            place.origin[0] as f64 * k,
            place.origin[1] as f64 * k,
            place.rotation,
            match (fp, place.mirror) {
                (true, true) => "bottom",
                (true, false) => "top",
                (false, true) => "mirrored",
                (false, false) => "as seen",
            }
        );
        SubPane::new("Placement")
            .id_salt("fp placement")
            .summary(summary)
            .tooltip(
                "Where the footprint, or the model view that moves, sits on the page: its origin, its turn, which side is seen",
            )
            .show(ui, |ui| self.placement_controls(ui));
    }

    fn placement_controls(&mut self, ui: &mut Ui) {
        let keys = self.keys();
        let k = self.mm_per_point();
        let fp = self.model_views.moves == modelviews::Moves::Footprint;
        let what = self.moving_name();
        let mut place = self.moving_mut().clone();
        if !fp {
            hint(
                ui,
                format!(
                    "Placing {what}; {} moves the footprint again (Model views)",
                    key_name(keys.moves)
                ),
            );
        }
        PropertyRow::new("Position")
            .tooltip(format!(
                "The origin of {what} from the page's top left corner, in mm at the drawing's scale"
            ))
            .show(ui, |ui| {
                let mut xy = place.origin.map(|v| v as f64 * k);
                let [x, y] = &mut xy;
                if ui
                    .add(
                        VecField::new([x, y])
                            .unit("mm")
                            .speed(0.01)
                            .decimals(3)
                            .tooltip("Across and down the page; the arrow keys nudge it too"),
                    )
                    .changed()
                    && k > 0.0
                {
                    place.origin = xy.map(|v| (v / k) as f32);
                }
            });
        PropertyRow::new("Rotation")
            .tooltip("Counter-clockwise on screen, degrees")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 70.0).max(60.0);
                    ui.add(
                        ScalarField::new(&mut place.rotation)
                            .unit("°")
                            .speed(0.5)
                            .decimals(1)
                            .range(-360.0..=360.0)
                            .width(w)
                            .tooltip(format!("The turn of {what}")),
                    );
                    if ui
                        .button("⟲ 90°")
                        .on_hover_text(format!("A quarter turn; {}", key_name(keys.rotate)))
                        .clicked()
                    {
                        place.rotation = (place.rotation + 90.0).rem_euclid(360.0);
                    }
                });
            });
        let mut centre = false;
        ui.horizontal_wrapped(|ui| {
            let (text, tip) = if fp {
                ("Bottom view", "Mirrored, as seen from below")
            } else {
                ("Mirrored", "Mirrored left to right")
            };
            ui.add(
                ToggleChip::new(&mut place.mirror, text)
                    .tooltip(format!("{tip}; {}", key_name(keys.mirror))),
            );
            centre = ui
                .button("Centre in view")
                .on_hover_text(format!("{what} into the middle of the view"))
                .clicked();
        });
        *self.moving_mut() = place;
        if centre {
            match self.model_views.moves {
                modelviews::Moves::Model => self.centre_model(),
                modelviews::Moves::Footprint => self.rt.center_footprint = true,
            }
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // Review

    fn review_section(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let row = self
            .rt
            .target
            .as_ref()
            .and_then(|t| self.symbol_row(&t.symbol))
            .and_then(|i| self.rt.symbols.as_ref().map(|l| l.rows[i].status));
        let summary = match (row, &self.rt.accepted) {
            (_, Some(_)) => "accepted".to_string(),
            (Some(s), None) => s.name().to_string(),
            (None, None) => "nothing to accept".into(),
        };
        SubPane::new("Review")
            .id_salt("fp review")
            .summary(summary)
            .tooltip("The symbol's review lines, its MPNs, what Accept wrote and why it is off")
            .show(ui, |ui| self.review_controls(ui, ctx));
    }

    fn review_controls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if let Some(why) = self.accept_blocker() {
            hint(ui, why);
        }
        self.review_lines_ui(ui);
        if let Some(h) = &self.rt.accepted {
            ui.label(format!("Accepted. {FIELD} = {h} (copied)"));
            ui.horizontal(|ui| {
                ui.label(RichText::new(h).monospace());
                if ui
                    .small_button("Copy")
                    .on_hover_text("The review hash to the clipboard again")
                    .clicked()
                {
                    ctx.copy_text(h.clone());
                }
            });
        }
        match &self.rt.written {
            Some(Ok(run)) => {
                for a in &run.symbols {
                    ui.label(format!("{}: written into", a.symbol));
                    for w in &a.files {
                        let kind = match w.source {
                            HashSource::Library => "library",
                            HashSource::Schematic => "schematic copy",
                        };
                        let what = if w.changed { "" } else { ", had it already" };
                        let name = file_name(&w.file);
                        ui.add(
                            Label::new(RichText::new(format!("  {name} ({kind}{what})")).small())
                                .truncate(),
                        )
                        .on_hover_text(w.file.display().to_string());
                    }
                    if let Some(n) = &a.library_note {
                        notice(ui, Tone::Warn, format!("Library symbol not written: {n}"));
                    }
                }
            }
            Some(Err(e)) => {
                notice(ui, Tone::Crit, format!("Not written into the symbols: {e}"));
            }
            None => {}
        }
        let n = self.reviews.len();
        SubPane::new(format!("Accepted reviews ({n})"))
            .id_salt("reviews")
            .tooltip("The reviews accepted in this session, newest first")
            .default_open(false)
            .show(ui, |ui| {
                if self.reviews.is_empty() {
                    hint(ui, "none yet in this session");
                }
                for r in self.reviews.iter().rev() {
                    let ds = r.datasheet.as_deref().map(file_name).unwrap_or_default();
                    ui.add(
                        Label::new(format!("{}  {}  page {}  {ds}", r.lib_id, r.hash, r.page))
                            .truncate(),
                    );
                }
            });
    }

    /// The symbol's review lines with their status, the MPNs no line reviews yet, and the review record's note
    fn review_lines_ui(&self, ui: &mut Ui) {
        let rec = self.recorded();
        if !rec.lines.is_empty() {
            Group::new("Reviews")
                .tooltip("The review lines in the symbol's Footprint_Review field")
                .show(ui, |ui| {
                    for l in &rec.lines {
                        let (word, _) = line_status(l.status);
                        let mpns = if l.line.mpns.is_empty() {
                            "every MPN".to_string()
                        } else {
                            l.line.mpns.join(", ")
                        };
                        let tone = line_tone(l.status);
                        let t = ve_theme::Tokens::of_ui(ui);
                        let ink = t.legible(tone.ink(&t), ui.visuals().panel_fill);
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(ink, word);
                            ui.label(format!("{}: {mpns}", line_source(&l.line)));
                        })
                        .response
                        .on_hover_text(format!("{}: {}", l.property, l.value));
                    }
                });
        }
        if !rec.unreviewed_mpns.is_empty() {
            notice(
                ui,
                Tone::Warn,
                format!(
                    "MPNs not reviewed yet: {}: accept against their datasheet",
                    rec.unreviewed_mpns.join(", ")
                ),
            );
        }
        if rec.whole_unreviewed {
            notice(
                ui,
                Tone::Warn,
                "the symbol has no Mpn fields, and every review line names MPNs: none reviews it as a whole",
            );
        }
        if let Some(n) = &self.rt.record_note {
            notice(ui, Tone::Accent, n);
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // Notes

    fn notes_section(&mut self, ui: &mut Ui) {
        let file = self.notes_file();
        let summary = file.as_deref().map_or("no notes file".into(), file_name);
        SubPane::new("Notes")
            .id_salt("fp notes")
            .summary(summary)
            .tooltip("The review notes: written above the canvas, appended to a Markdown file")
            .show(ui, |ui| {
                Facts::new("notes").show(ui, |f| {
                    match &file {
                        Some(p) => f.fact("File", file_name(p), p.display().to_string()),
                        None => f.fact("File", "(none)", "Settings: review notes file"),
                    };
                    match &self.rt.note_written {
                        Some(Ok(_)) => f.fact_tone("Last note", "written", Tone::Good, ""),
                        Some(Err(_)) => {
                            f.fact_tone("Last note", "not written: see above the canvas", Tone::Crit, "")
                        }
                        None => f.fact("Last note", "none yet", ""),
                    };
                });
                if let Some(why) = self.note_blocker() {
                    hint(ui, format!("Adding a note: {why}"));
                }
                hint(
                    ui,
                    format!(
                        "{} writes a note above the canvas; the measuring rulers add M1, M2, …; Enter adds it to \
                         the file with the symbol, the footprint, the datasheet and its page",
                        key_name(self.keys().note)
                    ),
                );
            });
    }

    // -----------------------------------------------------------------------------------------------------
    // Settings

    fn settings_section(&mut self, ui: &mut Ui) {
        let file = self.rt.settings_file.clone();
        SubPane::new("Settings")
            .id_salt("fp settings")
            .summary(file.as_deref().map_or("not saved".into(), file_name))
            .tooltip("The review's settings: display, ruler, keys, commands, the review record")
            .default_open(false)
            .show(ui, |ui| self.settings_controls(ui, file));
    }

    fn settings_controls(&mut self, ui: &mut Ui, file: Option<PathBuf>) {
        let s = &mut self.rt.settings;
        Group::new("Display").show(ui, |ui| {
            PropertyRow::new("Blink").show(ui, |ui| {
                ui.add(
                    ScalarField::new(&mut s.blink_hz)
                        .range(0.2..=20.0)
                        .speed(0.05)
                        .decimals(1)
                        .unit("/s")
                        .tooltip("Times a second the footprint blinks"),
                )
            });
            PropertyRow::new("Opacity at first").show(ui, |ui| {
                ui.add(
                    ScalarField::new(&mut s.opacity)
                        .range(0.0..=1.0)
                        .speed(0.005)
                        .tooltip("The footprint's opacity when a review starts"),
                )
            });
            PropertyRow::new("Finest drawing").show(ui, |ui| {
                ui.add(
                    ScalarField::new(&mut s.max_render_scale)
                        .range(1.0..=32.0)
                        .speed(0.05)
                        .decimals(1)
                        .unit("px/pt")
                        .tooltip("The page is drawn at most this many pixels per PDF point when zoomed in"),
                )
            });
        });
        Group::new("Ruler").show(ui, |ui| {
            PropertyRow::new("End zoom").show(ui, |ui| {
                ui.add(
                    ScalarField::new(&mut s.ruler_zoom)
                        .range(1.0..=64.0)
                        .speed(0.05)
                        .decimals(1)
                        .unit("×")
                        .tooltip("Zoom on a ruler end, times the view"),
                )
            });
            PropertyRow::new("Snaps to lines").show(ui, |ui| {
                ui.add(
                    ScalarField::new(&mut s.line_snap_px)
                        .range(0.0..=50.0)
                        .speed(0.1)
                        .decimals(1)
                        .unit("px")
                        .tooltip(
                            "a line's end, two lines crossing, else onto the line, this near on screen; 0: never; \
                             Alt held: free",
                        ),
                )
            });
            ui.add(
                ToggleChip::new(&mut s.ruler_snap, "Snaps level")
                    .tooltip("horizontal or vertical; Shift held: any angle"),
            );
        });
        Group::new("DXF").show(ui, |ui| {
            PropertyRow::new("Curve tolerance").show(ui, |ui| {
                ui.add(
                    ScalarField::new(&mut s.dxf_tolerance_mm)
                        .range(0.0001..=0.5)
                        .speed(0.0005)
                        .decimals(4)
                        .unit("mm")
                        .tooltip(
                            "a curve of the drawing within this of a circle is an arc in the DXF, any other is made \
                             of straight pieces within it",
                        ),
                )
            });
        });
        Group::new("Keys over the canvas")
            .tooltip("By name: F, Space, F5, …")
            .show(ui, |ui| {
                let bad = ui.visuals().error_fg_color;
                Grid::new("fp keys grid").num_columns(2).show(ui, |ui| {
                    let k = &mut s.keys;
                    for (what, name) in [
                        ("Page at full width", &mut k.full_width),
                        ("Move: the mouse moves the footprint", &mut k.move_tool),
                        ("Ruler: one end, then the other", &mut k.ruler),
                        ("Measure PDF", &mut k.measure_pdf),
                        ("Measure footprint", &mut k.measure_footprint),
                        ("Zoom on a ruler end, the other, back", &mut k.ruler_zoom),
                        ("Type the ruler's length", &mut k.scale),
                        ("The dimension's number sets the scale", &mut k.dimension),
                        ("Turn 90°", &mut k.rotate),
                        ("Bottom view", &mut k.mirror),
                        ("Blink", &mut k.blink),
                        ("Cycles copper/mask/paste", &mut k.cu_layer),
                        ("Invert the page", &mut k.invert),
                        ("What the SpaceMouse's puck moves", &mut k.puck),
                        ("Accept", &mut k.accept),
                        ("Next to review", &mut k.next),
                        ("Write a review note", &mut k.note),
                        ("Select the drawing's lines", &mut k.select),
                        ("Save the selection as DXF", &mut k.save_dxf),
                        ("The page or the 3D model", &mut k.view_3d),
                        ("Measure model", &mut k.measure_model),
                        (
                            "Measure across, footprint to drawing",
                            &mut k.measure_across,
                        ),
                        ("X-ray: the 3D model see-through", &mut k.xray),
                        ("Hide or show the footprint", &mut k.hide_footprint),
                        ("Hide or show the model", &mut k.hide_model),
                        ("Held: the page alone", &mut k.peek),
                    ] {
                        ui.label(what);
                        let ok = egui::Key::from_name(name.trim()).is_some();
                        let mut edit = TextEdit::singleline(name).desired_width(70.0);
                        if !ok {
                            edit = edit.text_color(bad);
                        }
                        let r = ui.add(edit);
                        if !ok {
                            r.on_hover_text("not a key name: this step has no key");
                        }
                        ui.end_row();
                    }
                });
            });
        Group::new("Page search and fonts").show(ui, |ui| {
            PropertyRow::new("Page search words, best first, one per line").show(ui, |ui| {
                if ui
                    .add(TextEdit::multiline(&mut self.rt.words_text).desired_rows(4))
                    .changed()
                {
                    s.page_search_words = lines(&self.rt.words_text);
                    self.rt.page_hits = None;
                }
            });
            PropertyRow::new("Font folders (Liberation fonts), one per line").show(ui, |ui| {
                if ui
                    .add(TextEdit::multiline(&mut self.rt.fonts_text).desired_rows(2))
                    .changed()
                {
                    s.font_dirs = lines(&self.rt.fonts_text)
                        .into_iter()
                        .map(PathBuf::from)
                        .collect();
                }
            });
        });
        Group::new("Library commands").show(ui, |ui| {
            PropertyRow::new("Library search command (search words are added)").show(ui, |ui| {
                if ui
                    .add(TextEdit::singleline(&mut self.rt.search_text))
                    .changed()
                {
                    s.library_search = self
                        .rt
                        .search_text
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                }
            });
            PropertyRow::new("Library name command, for the notes (the datasheet's file is added)")
                .show(ui, |ui| {
                    if ui
                        .add(TextEdit::singleline(&mut self.rt.name_text))
                        .changed()
                    {
                        s.library_name = self
                            .rt
                            .name_text
                            .split_whitespace()
                            .map(String::from)
                            .collect();
                        self.rt.doc_name = None;
                    }
                });
            PropertyRow::new(
                "Library docs command, for the review record (the index with content hashes)",
            )
            .show(ui, |ui| {
                if ui
                    .add(TextEdit::singleline(&mut self.rt.docs_text))
                    .changed()
                {
                    s.library_docs = self
                        .rt
                        .docs_text
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                    self.rt.doc_index = None;
                }
            });
            PropertyRow::new(
                "Library MPN lookup command, for the table of MPNs (the MPNs are added)",
            )
            .show(ui, |ui| {
                if ui
                    .add(TextEdit::singleline(&mut self.rt.mpn_text))
                    .changed()
                {
                    s.library_mpn = self
                        .rt
                        .mpn_text
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                    self.rt.mpn_docs.known.clear();
                }
            });
            PropertyRow::new(
                "Library fetch command, for an MPN without a document (the MPN is added)",
            )
            .show(ui, |ui| {
                if ui
                    .add(TextEdit::singleline(&mut self.rt.fetch_text))
                    .changed()
                {
                    s.library_fetch = self
                        .rt
                        .fetch_text
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                }
            });
        });
        Group::new("Review record").show(ui, |ui| {
            PropertyRow::new("Reviewer, written into the review record; empty: the login name")
                .show(ui, |ui| ui.add(TextEdit::singleline(&mut s.reviewer)));
            PropertyRow::new("Review notes file: a path, or a name beside the project or library")
                .show(ui, |ui| ui.add(TextEdit::singleline(&mut s.notes_file)));
        });
        ui.horizontal(|ui| {
            let why = file
                .is_none()
                .then_some("neither BERC_CONFIG, XDG_CONFIG_HOME nor HOME is set");
            let tip = file
                .as_ref()
                .map(|f| f.display().to_string())
                .unwrap_or_default();
            if button_if(ui, "Save", &tip, why).clicked()
                && let Some(f) = &file
            {
                let (mut all, _) = Settings::load_from(f);
                all.footprint_review = self.rt.settings.clone();
                self.rt.settings_note = Some(match all.save_to(f) {
                    Ok(()) => format!("saved to {}", f.display()),
                    Err(e) => e,
                });
                self.usage("settings-save");
            }
            if ui
                .button("Defaults")
                .on_hover_text("Every setting above back to its default (Save keeps them)")
                .clicked()
            {
                self.set_settings(FootprintReviewSettings::default());
                self.rt.page_hits = None;
            }
        });
        if let Some(n) = &self.rt.settings_note {
            hint(ui, n);
        }
    }
}
