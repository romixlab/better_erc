//! The footprint's centre (VIEW-35): a thin cross with long arms along the footprint's own X and Y (their reach a
//! setting), and while the footprint moves (a drag, the arrows, the puck, carried by the puck under a drag) its
//! centre snaps to the drawing's lines as a ruler end does: a line's end, two lines crossing, else onto a line. The
//! snap target is drawn as the ruler's is. A switch (`.` and a button in the mode bar, on by default) turns it off;
//! Alt held frees a move.
//!
//! The move keeps where the footprint would be unsnapped, so a drag or a run of arrow presses pulls it off a line
//! again once it is out of the snap's reach; anything else that moves the footprint starts afresh from where it is.

use super::*;

/// How long the snap target stays shown after the footprint last moved, seconds
const SHOWN_S: f64 = 0.6;

/// The cross' colour
const CROSS_COLOUR: Color32 = Color32::from_rgb(0, 160, 0);

impl FootprintReview {
    /// Whether the footprint's centre snaps to the drawing's lines while it moves
    pub(super) fn centre_snap_on(&self) -> bool {
        self.centre_snap.unwrap_or(self.rt.settings.centre_snap)
    }

    /// The centre snap on or off
    pub(super) fn toggle_centre_snap(&mut self) {
        let on = !self.centre_snap_on();
        self.centre_snap = Some(on);
        self.rt.centre_snapped = None;
        self.usage(if on {
            "centre snap on"
        } else {
            "centre snap off"
        });
    }

    /// A move of the footprint starts here: the next one goes from where the footprint is, not from where an
    /// earlier move would have left it unsnapped
    pub(super) fn centre_restart(&mut self) {
        self.rt.centre_free = None;
    }

    /// The footprint moved by `d` page points at time `now`: from where it would be unsnapped, its centre snapped to
    /// the drawing's lines unless the switch is off or `free` (Alt held)
    pub(super) fn move_footprint(&mut self, d: [f32; 2], free: bool, now: f64) {
        let from = match self.rt.centre_free {
            Some((unsnapped, set)) if set == self.place.origin => unsnapped,
            _ => self.place.origin,
        };
        let to = [from[0] + d[0], from[1] + d[1]];
        let snap = if self.centre_snap_on() && !free {
            self.line_snap(to, false)
        } else {
            None
        };
        self.place.origin = snap.as_ref().map_or(to, |s| s.at);
        self.rt.centre_free = Some((to, self.place.origin));
        self.rt.centre_snapped = snap.map(|s| (s, now));
    }

    /// The centre cross: thin, its arms along the footprint's X and Y as far as the setting says
    pub(super) fn draw_centre(&self, painter: &egui::Painter, canvas: Rect) {
        let o = self.to_screen(canvas, self.place.origin);
        let arm = self.rt.settings.centre_cross_px.max(1.0);
        let s = Stroke::new(1.0, CROSS_COLOUR);
        for axis in [[1.0, 0.0], [0.0, 1.0]] {
            // The footprint's axis on screen, turned and mirrored with it
            let d = (self.to_screen(canvas, self.fp_to_page(axis)) - o).normalized();
            if d.is_finite() {
                painter.line_segment([o - d * arm, o + d * arm], s);
            }
        }
    }

    /// What the centre snapped to while the footprint moves, and a moment after, drawn as a ruler end's snap
    pub(super) fn draw_centre_snap(&self, painter: &egui::Painter, canvas: Rect, now: f64) {
        let Some((snap, t)) = &self.rt.centre_snapped else {
            return;
        };
        let left = SHOWN_S - (now - t);
        if left <= 0.0 {
            return;
        }
        painter
            .ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(left));
        self.draw_snap_mark(painter, canvas, snap);
    }
}
