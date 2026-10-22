use crate::prelude::*;
use crate::tabs::{TabUi, severity_color};

#[derive(Default, Serialize, Deserialize)]
pub struct Style {
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {}

/// The findings of the style checks: values and notation (BOM), symbol pin names (LIB)
fn is_style(check: &str) -> bool {
    check.starts_with("BOM-") || check.starts_with("LIB-")
}

impl TabUi for Style {
    fn init(&mut self, _cx: &Context) {
        self.transient = Some(Transient {});
    }

    fn ui(&mut self, ui: &mut Ui, cx: &mut Context, _id: Id) {
        let Some(_t) = &mut self.transient else {
            return;
        };
        let s = cx.blocking_read();
        ScrollArea::vertical().show(ui, |ui| {
            for d in &s.designs {
                let mut any = false;
                for f in d.run.report.findings.iter().filter(|f| is_style(&f.check)) {
                    any = true;
                    let subjects: Vec<&str> = f.subjects.iter().map(|s| s.name.as_str()).collect();
                    ui.colored_label(
                        severity_color(f.severity),
                        format!("{} {}: {}", f.check, subjects.join(", "), f.message),
                    );
                }
                if !any {
                    ui.label("No style warnings found");
                }
            }
        });
    }
}
