use crate::prelude::*;
use crate::tabs::TabUi;
use ecad_file_format::load_altium_netlist;
use ecad_file_format::orcad_netlist::Rule::net;
use ecad_file_format::pcb_assembly::PcbAssembly;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Default, Serialize, Deserialize)]
pub struct PcbDataImport {
    source: Source,
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {}

#[derive(Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(derive(AsRefStr, EnumIter))]
#[strum_discriminants(name(SourceKind))]
enum Source {
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
        Source::KiCadSchematic { path: None }
    }
}

impl Source {
    fn turn_into(&mut self, kind: SourceKind) {
        *self = match kind {
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
}

impl TabUi for PcbDataImport {
    fn init(&mut self, _cx: &Context) {
        self.transient = Some(Transient {});
    }

    fn ui(&mut self, ui: &mut Ui, cx: &mut Context, id: Id) {
        let Some(_t) = &mut self.transient else {
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
            match &self.source {
                Source::KiCadSchematic { .. } => {}
                Source::AltiumNetlist {
                    edif_path,
                    wirelist_path,
                } => {
                    if let (Some(edif_path), Some(wirelist_path)) = (edif_path, wirelist_path) {
                        let netlist = load_altium_netlist(edif_path, wirelist_path);
                        match netlist {
                            Ok(netlist) => {
                                cx.blocking_write().boards.push(PcbAssembly {
                                    name: Arc::new("".to_string()),
                                    netlist,
                                    pnp: Default::default(),
                                    bom: (),
                                });
                            }
                            Err(e) => {
                                error!("{e:?}");
                            }
                        }
                    }
                }
                Source::OrcadNetlist { .. } => {}
            }
        }
    }
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
        if ui.button("Choose").clicked() {
            if let Some(p) = FileDialog::new().pick_file() {
                *path = Some(p);
                changed = true;
            }
        }
    });
    changed
}
