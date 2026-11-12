//! The SpaceMouse in the footprint review (VIEW-20), through spacenavd ([`ve_spacemouse`]): the puck moves the page
//! (or the footprint, [`PuckMoves`]) and zooms the view; its two buttons open pie menus with the canvas' actions,
//! each slot named with its key, a disabled one saying why. The canvas' keys and the pie's slots are the same
//! [`Action`]s, done by [`FootprintReview::act`].
//!
//! Pan is in screen points per second: a push moves the page across the screen at the same speed at every zoom.
//! The mapping (which raw axis does what, the speed, which button is which) is the app's: `[spacemouse]` in the
//! settings file, measured by the calibrate step reached from the tab.

use super::*;
use ve_spacemouse::{
    Axes, Calibration, CalibrationAction, Event, Mapping, PieMenu, PieState, Slot, calibration_ui,
    icons, mapping_ui,
};

/// What the canvas does on a key or a pie slot
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    FullWidth,
    Ruler,
    RulerZoom,
    Scale,
    Dimension,
    /// Out of the ruler step and back to Move (Esc)
    EndRuler,
    /// Into a mode; its own key again goes back to Move ([`FootprintReview::tool_key`])
    Mode(Tool),
    Rotate,
    Mirror,
    Blink,
    CuLayer,
    Invert,
    PageUp,
    PageDown,
    /// What the puck moves: the page or the footprint
    Puck,
    Accept,
    NextToReview,
    /// The review note field takes the keyboard
    Note,
    /// Accept's question answered: write, or nothing
    Write,
    Cancel,
}

/// What the puck's slide moves; its push and pull always zoom the view
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PuckMoves {
    /// The page under the footprint and all: the view
    #[default]
    Page,
    /// The footprint over the page
    Footprint,
}

/// The SpaceMouse's state in the tab: never saved
pub(super) struct Puck {
    /// The socket to connect to on the first frame; none: no SpaceMouse (the tests that don't fake one)
    pub socket: Option<PathBuf>,
    #[cfg(unix)]
    mouse: Option<ve_spacemouse::SpaceMouse>,
    /// The deflection this frame
    pub axes: Axes,
    pub mapping: Mapping,
    pub pie: PieState,
    pub calibration: Option<Calibration>,
    /// Where the mapping was saved, or why not
    pub note: Option<String>,
}

impl Default for Puck {
    fn default() -> Self {
        Puck {
            socket: cfg!(unix).then(|| PathBuf::from("/var/run/spnav.sock")),
            #[cfg(unix)]
            mouse: None,
            axes: Axes::default(),
            mapping: Mapping::default(),
            pie: PieState::default(),
            calibration: None,
            note: None,
        }
    }
}

impl Puck {
    /// spacenavd's events since the last frame; the client started on the first call
    fn poll(&mut self, ctx: &egui::Context) -> Vec<Event> {
        #[cfg(unix)]
        {
            if self.mouse.is_none()
                && let Some(socket) = &self.socket
            {
                let mouse = ve_spacemouse::SpaceMouse::start(ve_spacemouse::Options {
                    socket: socket.clone(),
                    ..Default::default()
                });
                self.mouse = Some(mouse.with_repaint(ctx));
            }
            if let Some(m) = &self.mouse {
                self.axes = m.axes();
                return m.poll();
            }
        }
        let _ = ctx;
        Vec::new()
    }

    /// One line on the connection, and whether the SpaceMouse is there
    fn status(&self) -> (String, bool) {
        #[cfg(unix)]
        if let Some(m) = &self.mouse {
            let s = m.status();
            return (s.line(), s == ve_spacemouse::Status::Connected);
        }
        ("no SpaceMouse in this build".into(), false)
    }

    /// The socket the client reads, for the status' tooltip
    fn socket_line(&self) -> String {
        self.socket
            .as_ref()
            .map_or("none".into(), |s| s.display().to_string())
    }
}

/// A key as a pie slot shows it after the action's name
fn short_key(k: Option<egui::Key>) -> String {
    k.map_or_else(|| "no key".into(), |k| k.symbol_or_name().to_string())
}

impl FootprintReview {
    /// The first part of a frame: spacenavd's events, fed to the calibrate step while it is open
    pub(super) fn puck_poll(&mut self, ctx: &egui::Context) -> Vec<Event> {
        let events = self.rt.puck.poll(ctx);
        if let Some(cal) = &mut self.rt.puck.calibration {
            let now = ctx.input(|i| i.time);
            for e in &events {
                cal.feed(e, now);
            }
        }
        events
    }

    /// The puck this frame: its slide pans the page or moves the footprint in screen points, so a push goes as fast
    /// at every zoom; its push and pull zoom the view about the pointer over the canvas, else the canvas' middle.
    /// Nothing while the calibrate step measures the motions.
    pub(super) fn puck_nav(&mut self, ctx: &egui::Context, canvas: Rect, pointer: Option<Pos2>) {
        if self.rt.puck.calibration.is_some() {
            return;
        }
        let nav = self.rt.puck.mapping.apply(&self.rt.puck.axes);
        if nav.is_zero() {
            return;
        }
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let d = nav.pan_screen() * dt;
        match self.puck {
            PuckMoves::Page => {
                self.pan[0] += d.x;
                self.pan[1] += d.y;
            }
            PuckMoves::Footprint => {
                self.place.origin[0] += d.x / self.zoom;
                self.place.origin[1] += d.y / self.zoom;
            }
        }
        let factor = (nav.zoom * dt).exp();
        if factor != 1.0 {
            let at = pointer
                .filter(|p| canvas.contains(*p))
                .unwrap_or(canvas.center());
            let before = self.to_page(canvas, at);
            self.zoom = (self.zoom * factor).clamp(0.05, 400.0);
            let after = self.to_screen(canvas, before);
            self.pan[0] += at.x - after.x;
            self.pan[1] += at.y - after.y;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    /// The pie menus on the SpaceMouse's buttons, over everything; what fired is done at the place the menu
    /// opened. Not while the calibrate step records the buttons.
    pub(super) fn puck_pie(&mut self, ctx: &egui::Context, events: &[Event]) {
        if self.rt.puck.calibration.is_some() {
            return;
        }
        let menu = self.pie_menu();
        let buttons = ve_spacemouse::buttons(events, &self.rt.puck.mapping);
        let fired = menu.show(ctx, &mut self.rt.puck.pie, &buttons);
        let (free, lines) = ctx.input(|i| (i.modifiers.shift, !i.modifiers.alt));
        for f in fired {
            let at = self.to_page(self.rt.canvas, f.at);
            self.act(ctx, f.action, Some(at), free, lines);
            self.usage("puck pie");
        }
    }

    /// The pie menus: the left button's ruler and page, the right one's footprint, both for Accept and the next
    /// symbol; a tap of the left button lays the ruler, of the right one switches what the puck moves. While
    /// Accept's question is open, the taps answer it and every ring slot says so.
    pub(super) fn pie_menu(&self) -> PieMenu<Action> {
        use Action::*;
        let k = self.keys();
        let slot = |name: &str, key: String, icon: &str, a: Action| {
            Slot::new(format!("{name} · {key}"), icon, a)
        };
        let pdf = self.rt.pdf.as_ref().filter(|p| p.page_count() > 0);
        let no_pdf = "Open the datasheet first (left)";
        let no_fp = "No footprint shown: pick a symbol (left)";
        let no_ruler = format!(
            "No ruler yet: {} lays one, or the left button's tap",
            short_key(k.ruler)
        );
        let pages = pdf.map_or(0, |p| p.page_count());
        let measuring = format!(
            "Measuring: the scale is set in the Scale mode ({})",
            short_key(k.ruler)
        );
        let dimension = if self.tool.measures() {
            Some(measuring.clone())
        } else if self.ruler.is_none() {
            Some(no_ruler.clone())
        } else if self.dimension_number().is_none() {
            Some(format!(
                "No number printed between the ruler's ends: {} types its length",
                short_key(k.scale)
            ))
        } else {
            None
        };
        let footprint = |s: Slot<Action>| s.disabled_if(self.rt.items.is_empty(), no_fp);
        let datasheet = |s: Slot<Action>| s.disabled_if(pdf.is_none(), no_pdf);
        let mut menu = PieMenu {
            center_left: Some(datasheet(slot(
                "Ruler",
                short_key(k.ruler),
                icons::RULER,
                Ruler,
            ))),
            center_right: Some(slot(
                match self.puck {
                    PuckMoves::Page => "Move footprint",
                    PuckMoves::Footprint => "Move page",
                },
                short_key(k.puck),
                icons::ARROWS_OUT_CARDINAL,
                Puck,
            )),
            // Clockwise from the top
            ring_left: vec![
                Some(datasheet(slot(
                    "Full width",
                    short_key(k.full_width),
                    icons::ARROWS_OUT_LINE_HORIZONTAL,
                    FullWidth,
                ))),
                Some(
                    slot(
                        "Zoom on a ruler end",
                        short_key(k.ruler_zoom),
                        icons::MAGNIFYING_GLASS_PLUS,
                        RulerZoom,
                    )
                    .disabled_if(self.ruler.is_none(), no_ruler.clone()),
                ),
                Some(
                    slot("Type the length", short_key(k.scale), icons::TEXTBOX, Scale)
                        .disabled_if(self.ruler.is_none(), no_ruler)
                        .disabled_if(self.tool.measures(), measuring),
                ),
                Some(Slot {
                    disabled: dimension,
                    ..slot(
                        "The dimension's number",
                        short_key(k.dimension),
                        icons::NUMBER_SQUARE_TWO,
                        Dimension,
                    )
                }),
                Some(
                    slot(
                        "Back to Move",
                        short_key(k.move_tool),
                        icons::HAND_GRABBING,
                        Mode(Tool::Move),
                    )
                    .disabled_if(
                        self.tool == Tool::Move,
                        "In Move already: the mouse moves the footprint",
                    ),
                ),
                Some(datasheet(slot(
                    if self.tool == Tool::MeasurePdf {
                        "Stop measuring the PDF"
                    } else {
                        "Measure the PDF"
                    },
                    short_key(k.measure_pdf),
                    icons::RULER,
                    Mode(Tool::MeasurePdf),
                ))),
                Some(
                    datasheet(slot(
                        "Next page",
                        "PgDn".into(),
                        icons::ARROW_FAT_DOWN,
                        PageDown,
                    ))
                    .disabled_if(
                        pdf.is_some() && self.page + 1 >= pages,
                        "This is the last page",
                    ),
                ),
                Some(datasheet(slot(
                    "Invert the page",
                    short_key(k.invert),
                    icons::CIRCLE_HALF,
                    Invert,
                ))),
                Some(
                    datasheet(slot(
                        "Previous page",
                        "PgUp".into(),
                        icons::ARROW_FAT_UP,
                        PageUp,
                    ))
                    .disabled_if(pdf.is_some() && self.page == 0, "This is the first page"),
                ),
            ],
            ring_right: vec![
                Some(footprint(slot(
                    "Turn 90°",
                    short_key(k.rotate),
                    icons::ARROW_CLOCKWISE,
                    Rotate,
                ))),
                Some(footprint(slot(
                    "Bottom view",
                    short_key(k.mirror),
                    icons::FLIP_HORIZONTAL,
                    Mirror,
                ))),
                Some(footprint(slot(
                    if self.blink { "Stop blinking" } else { "Blink" },
                    short_key(k.blink),
                    icons::EYE,
                    Blink,
                ))),
                Some(footprint(slot(
                    "Copper, mask, paste",
                    short_key(k.cu_layer),
                    icons::STACK,
                    CuLayer,
                ))),
                Some(footprint(slot(
                    if self.tool == Tool::MeasureFootprint {
                        "Stop measuring the footprint"
                    } else {
                        "Measure the footprint"
                    },
                    short_key(k.measure_footprint),
                    icons::RULER,
                    Mode(Tool::MeasureFootprint),
                ))),
            ],
            ring_both: vec![
                Some(
                    slot("Accept", short_key(k.accept), icons::CHECK_CIRCLE, Accept).disabled_if(
                        self.accept_blocker().is_some(),
                        self.accept_blocker().unwrap_or_default(),
                    ),
                ),
                Some(
                    slot(
                        "Next to review",
                        short_key(k.next),
                        icons::SKIP_FORWARD,
                        NextToReview,
                    )
                    .disabled_if(
                        self.next_to_review().is_none(),
                        if self.rt.symbols.is_none() {
                            "No symbol list open: open a library or a board (left)"
                        } else {
                            "Nothing else in the list needs a review"
                        },
                    ),
                ),
                Some(footprint(slot(
                    "Write a note",
                    short_key(k.note),
                    icons::NOTE_PENCIL,
                    Note,
                ))),
            ],
            free_hint: String::new(),
            ..Default::default()
        };
        if self.rt.confirm.is_some() {
            let why = "Accept's question is open: Write (Enter) or Cancel (Esc) first";
            for ring in [
                &mut menu.ring_left,
                &mut menu.ring_right,
                &mut menu.ring_both,
            ] {
                for s in ring.iter_mut().flatten() {
                    s.disabled = Some(why.into());
                }
            }
            menu.center_left = Some(Slot::new("Write · Enter", icons::FLOPPY_DISK, Write));
            menu.center_right = Some(Slot::new("Cancel · Esc", icons::X, Cancel));
        }
        menu
    }

    /// An action of the canvas, from its key or a pie slot; `at`: the place it is about (page points), `free`:
    /// Shift held, `lines`: Alt not held
    pub(super) fn act(
        &mut self,
        ctx: &egui::Context,
        action: Action,
        at: Option<[f32; 2]>,
        free: bool,
        lines: bool,
    ) {
        let pages = self.rt.pdf.as_ref().map_or(0, PdfDoc::page_count);
        match action {
            Action::FullWidth => {
                self.rt.fit_width = true;
                self.usage("full width");
            }
            Action::Ruler => {
                if let Some(at) = at {
                    self.ruler_key(at, free, lines);
                }
            }
            Action::RulerZoom => self.ruler_zoom_key(self.rt.canvas),
            Action::Scale => self.scale_key(),
            Action::Dimension => self.use_dimension_number(),
            Action::EndRuler => self.set_tool(Tool::Move),
            Action::Mode(t) => self.tool_key(t),
            Action::Rotate => self.place.rotation = (self.place.rotation + 90.0).rem_euclid(360.0),
            Action::Mirror => self.place.mirror = !self.place.mirror,
            Action::Blink => self.blink = !self.blink,
            Action::CuLayer => self.cycle_cu_layer(),
            Action::Invert => {
                self.invert = Some(!self.page_inverted());
                self.usage("invert");
            }
            Action::PageUp => {
                if self.page > 0 {
                    self.page -= 1;
                    self.rt.fit_page = true;
                }
            }
            Action::PageDown => {
                if self.page + 1 < pages {
                    self.page += 1;
                    self.rt.fit_page = true;
                }
            }
            Action::Puck => {
                self.puck = match self.puck {
                    PuckMoves::Page => PuckMoves::Footprint,
                    PuckMoves::Footprint => PuckMoves::Page,
                };
                self.usage("puck moves");
            }
            Action::Accept => {
                if self.rt.confirm.is_none() && self.accept_blocker().is_none() {
                    self.accept(ctx);
                }
            }
            Action::NextToReview => {
                if let Some(i) = self.next_to_review() {
                    self.select_symbol(i, true);
                    self.usage("next");
                }
            }
            Action::Note => {
                self.rt.focus_note = true;
                self.usage("note key");
            }
            Action::Write => self.confirmed(ctx, true),
            Action::Cancel => self.confirmed(ctx, false),
        }
    }

    /// The SpaceMouse in the controls: whether it is there, what the puck moves, Calibrate and the mapping
    pub(super) fn puck_controls(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("SpaceMouse")
            .id_salt("fp spacemouse")
            .default_open(false)
            .show(ui, |ui| {
                let (line, connected) = self.rt.puck.status();
                let colour = if connected {
                    Color32::from_rgb(0, 170, 60)
                } else {
                    ui.visuals().warn_fg_color
                };
                ui.colored_label(colour, line)
                    .on_hover_text(format!("spacenavd's socket: {}", self.rt.puck.socket_line()));
                ui.horizontal(|ui| {
                    ui.label("The puck moves");
                    let k = short_key(self.keys().puck);
                    ui.radio_value(&mut self.puck, PuckMoves::Page, "the page")
                        .on_hover_text(format!("its slide pans the view; {k} switches"));
                    ui.radio_value(&mut self.puck, PuckMoves::Footprint, "the footprint")
                        .on_hover_text(format!("its slide moves the footprint over the page; {k} switches"));
                });
                let calibrating = self.rt.puck.calibration.is_some();
                if ui
                    .add_enabled(!calibrating, Button::new("Calibrate…"))
                    .on_hover_text(
                        "Measure which way the puck reads when you slide it, lift it and push it away, and which \
                         button is which; saved for this app",
                    )
                    .on_disabled_hover_text("The calibration window is open")
                    .clicked()
                {
                    self.rt.puck.calibration =
                        Some(Calibration::two_d(self.rt.puck.mapping.clone()));
                    self.usage("puck calibrate");
                }
                egui::CollapsingHeader::new("Mapping")
                    .id_salt("fp spacemouse mapping")
                    .default_open(false)
                    .show(ui, |ui| {
                        let live = self.rt.puck.axes;
                        if mapping_ui(ui, &mut self.rt.puck.mapping, Some(&live)) {
                            self.rt.puck.note = Some("changed, not saved yet".into());
                        }
                        let file = self.rt.settings_file.clone();
                        let b = ui.add_enabled(file.is_some(), Button::new("Save the mapping"));
                        let b = match &file {
                            Some(f) => b.on_hover_text(f.display().to_string()),
                            None => b.on_disabled_hover_text(
                                "no settings file: neither BERC_CONFIG, XDG_CONFIG_HOME nor HOME is set",
                            ),
                        };
                        if b.clicked() {
                            self.save_mapping();
                        }
                    });
                if let Some(n) = &self.rt.puck.note {
                    ui.label(RichText::new(n).small());
                }
            });
    }

    /// The calibrate step's window while it is open; its mapping, once used, is the app's and saved
    pub(super) fn calibration_window(&mut self, ctx: &egui::Context, id: Id) {
        let Some(cal) = &mut self.rt.puck.calibration else {
            return;
        };
        let mut open = true;
        let mut action = CalibrationAction::None;
        egui::Window::new("Calibrate the SpaceMouse")
            .id(id.with("spacemouse calibration"))
            .open(&mut open)
            .default_width(460.0)
            .show(ctx, |ui| action = calibration_ui(ui, cal));
        match action {
            CalibrationAction::Apply(m) => {
                self.rt.puck.mapping = m;
                self.rt.puck.calibration = None;
                self.save_mapping();
                self.usage("puck calibrated");
            }
            CalibrationAction::Cancel => self.rt.puck.calibration = None,
            CalibrationAction::None if !open => self.rt.puck.calibration = None,
            CalibrationAction::None => {}
        }
    }

    /// The mapping into the settings file, `[spacemouse]`, the rest of it kept
    fn save_mapping(&mut self) {
        let Some(file) = self.rt.settings_file.clone() else {
            self.rt.puck.note = Some(
                "not saved: no settings file (neither BERC_CONFIG, XDG_CONFIG_HOME nor HOME is set)".into(),
            );
            return;
        };
        let (mut all, error) = Settings::load_from(&file);
        if let Some(e) = error {
            self.rt.puck.note = Some(format!("not saved, the file is left as it is: {e}"));
            return;
        }
        all.spacemouse = self.rt.puck.mapping.clone();
        self.rt.puck.note = Some(match all.save_to(&file) {
            Ok(()) => format!("saved to {}", file.display()),
            Err(e) => e,
        });
    }
}

#[cfg(test)]
#[path = "puck_tests.rs"]
mod tests;
