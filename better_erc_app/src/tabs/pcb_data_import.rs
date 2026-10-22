use crate::context::Loaded;
use crate::prelude::*;
use crate::tabs::{TabUi, severity_color};
use better_erc::{CheckError, Project, Status};
use rfd::FileDialog;
use std::path::PathBuf;

#[derive(Default, Serialize, Deserialize)]
pub struct PcbDataImport {
    source: Source,
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {
    /// The last load's error
    error: Option<String>,
}

#[derive(Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(derive(AsRefStr, EnumIter))]
#[strum_discriminants(name(SourceKind))]
enum Source {
    /// Any design the importer registry reads: a KiCad project, schematic, board or netlist, an Altium EDIF or
    /// WireList, an OrCAD netlist, a PnP file, or a directory with one design in it
    Design {
        path: Option<PathBuf>,
    },
    KiCadSchematic {
        path: Option<PathBuf>,
    },
    AltiumNetlist {
        edif_path: Option<PathBuf>,
        wirelist_path: Option<PathBuf>,
    },
    OrcadNetlist {
        lib_path: Option<PathBuf>,
        netlist_path: Option<PathBuf>,
        part_list_path: Option<PathBuf>,
    },
}

impl Default for Source {
    fn default() -> Self {
        Source::Design { path: None }
    }
}

impl Source {
    fn turn_into(&mut self, kind: SourceKind) {
        *self = match kind {
            SourceKind::Design => Source::Design { path: None },
            SourceKind::KiCadSchematic => Source::KiCadSchematic { path: None },
            SourceKind::AltiumNetlist => Source::AltiumNetlist {
                edif_path: None,
                wirelist_path: None,
            },
            SourceKind::OrcadNetlist => Source::OrcadNetlist {
                lib_path: None,
                netlist_path: None,
                part_list_path: None,
            },
        };
    }

    /// Opens the design the paths name, `None` until the main file is chosen
    fn open(&self) -> Option<Result<Project, CheckError>> {
        Some(match self {
            Source::Design { path } | Source::KiCadSchematic { path } => {
                Project::open(path.as_ref()?)
            }
            Source::AltiumNetlist {
                edif_path,
                wirelist_path,
            } => Project::open_files(
                edif_path.as_ref()?,
                &wirelist_path.iter().cloned().collect::<Vec<_>>(),
            ),
            Source::OrcadNetlist {
                lib_path,
                netlist_path,
                part_list_path,
            } => {
                let others: Vec<PathBuf> = [lib_path, part_list_path]
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect();
                Project::open_files(netlist_path.as_ref()?, &others)
            }
        })
    }
}

impl TabUi for PcbDataImport {
    fn init(&mut self, _cx: &Context) {
        self.transient = Some(Transient { error: None });
    }

    fn ui(&mut self, ui: &mut Ui, cx: &mut Context, id: Id) {
        let Some(t) = &mut self.transient else {
            return;
        };
        let mut new_kind = self.source.discriminant();
        ui.horizontal(|ui| {
            ComboBox::from_id_salt(id.with("Source"))
                .selected_text(self.source.discriminant().as_ref())
                .show_ui(ui, |ui| {
                    for kind in SourceKind::iter() {
                        ui.selectable_value(&mut new_kind, kind, kind.as_ref());
                    }
                });
        });
        if new_kind != self.source.discriminant() {
            self.source.turn_into(new_kind);
        }
        let mut changed = false;
        match &mut self.source {
            Source::Design { path } => {
                changed |= file_path("file", path, ui);
                ui.horizontal(|ui| {
                    ui.monospace("or directory");
                    if ui.button("Choose").clicked()
                        && let Some(p) = FileDialog::new().pick_folder()
                    {
                        *path = Some(p);
                        changed = true;
                    }
                });
            }
            Source::KiCadSchematic { path } => {
                changed |= file_path("root .kicad_sch", path, ui);
            }
            Source::AltiumNetlist {
                edif_path,
                wirelist_path,
            } => {
                changed |= file_path("EDIF", edif_path, ui);
                changed |= file_path("Wirelist", wirelist_path, ui);
            }
            Source::OrcadNetlist {
                lib_path,
                netlist_path,
                part_list_path,
            } => {
                changed |= file_path("Lib", lib_path, ui);
                changed |= file_path("Net list", netlist_path, ui);
                changed |= file_path("Part list", part_list_path, ui);
            }
        }
        let reload = ui.button("Reload").clicked();
        if changed || reload {
            match self.source.open().map(|p| p.and_then(Loaded::new)) {
                Some(Ok(loaded)) => {
                    t.error = None;
                    cx.blocking_write().load(loaded);
                }
                Some(Err(e)) => {
                    error!("{e}");
                    t.error = Some(e.to_string());
                }
                None => {}
            }
        }
        if let Some(e) = &t.error {
            ui.colored_label(Color32::RED, e);
        }
        ui.separator();
        let mut close = None;
        ScrollArea::vertical().show(ui, |ui| {
            let s = cx.blocking_read();
            for (i, d) in s.designs.iter().enumerate() {
                if report_ui(ui, id.with(i), d) {
                    close = Some(i);
                }
            }
        });
        if let Some(i) = close {
            cx.blocking_write().designs.remove(i);
        }
    }
}

/// A design's report: what was read, the findings with their check ids and explanations, the checks that did not
/// run and why. True when the design is to be closed.
fn report_ui(ui: &mut Ui, id: Id, d: &Loaded) -> bool {
    let report = &d.run.report;
    let mut close = false;
    ui.horizontal(|ui| {
        ui.heading(d.file.display().to_string());
        close = ui.button("Close").clicked();
    });
    for b in &d.design.boards {
        ui.label(format!(
            "{} ({}): {} parts, {} nets, {} sheets{}",
            b.name,
            d.importer,
            b.parts.len(),
            b.nets.len(),
            b.sheets.len(),
            if b.layout.is_some() { ", layout" } else { "" }
        ));
    }
    let sum = &report.summary;
    ui.label(format!(
        "{} errors, {} warnings, {} suggestions; {} checks ran, {} did not run, {} planned",
        sum.errors, sum.warnings, sum.suggestions, sum.ran, sum.not_run, sum.planned
    ));
    ui.collapsing(format!("Findings ({})", report.findings.len()), |ui| {
        for (i, f) in report.findings.iter().enumerate() {
            let subjects: Vec<&str> = f.subjects.iter().map(|s| s.name.as_str()).collect();
            egui::CollapsingHeader::new(
                RichText::new(format!("{} {}: {}", f.check, f.severity.name(), f.message))
                    .color(severity_color(f.severity)),
            )
            .id_salt(id.with(("finding", i)))
            .show(ui, |ui| {
                ui.label(format!(
                    "board {}, subjects: {}",
                    f.board,
                    subjects.join(", ")
                ));
                for fact in &f.facts {
                    ui.label(format!("{}: {}", fact.name, fact.value));
                }
                if let Some(fix) = &f.fix {
                    ui.label(format!("fix: {fix}"));
                }
                if let Some(e) = better_erc::explain(&f.check) {
                    ui.separator();
                    ui.label(e.to_text());
                }
            });
        }
    });
    let not_run: Vec<_> = report
        .checks
        .iter()
        .filter_map(|c| match &c.status {
            Status::NotRun(n) => Some(n),
            _ => None,
        })
        .collect();
    ui.collapsing(format!("Not run ({})", not_run.len()), |ui| {
        for n in not_run {
            ui.label(format!("{}: needs {}. {}", n.check, n.need, n.hint));
        }
    });
    ui.collapsing(format!("Notes ({})", report.notes.len()), |ui| {
        for n in &report.notes {
            ui.label(format!("{} {}: {}", n.kind, n.at, n.message));
        }
    });
    ui.separator();
    close
}

fn file_path(label: &str, path: &mut Option<PathBuf>, ui: &mut Ui) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.monospace(label);
        if let Some(path) = path {
            ui.label(path.display().to_string());
        } else {
            ui.label("-");
        }
        if ui.button("Choose").clicked()
            && let Some(p) = FileDialog::new().pick_file()
        {
            *path = Some(p);
            changed = true;
        }
    });
    changed
}
