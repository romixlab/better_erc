use crate::netlist::{
    Component, LibName, LibPart, LibPartName, Net, Netlist, Node, Pin, PinMode, PinType,
};
use crate::{Designator, NetName, PinId, PinName};
use anyhow::Result;
use serde::de::SeqAccess;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::read_to_string;
use std::path::PathBuf;

pub fn load_kicad_netlist(path: &PathBuf) -> Result<Netlist> {
    let contents = read_to_string(path)?;
    let netlist: KicadFileKind = serde_lexpr::from_str(&contents)?;
    let KicadFileKind::NetListExport(netlist) = netlist;

    let mut components = HashMap::new();
    for component in netlist.components {
        let ComponentKind::Component(component_entry) = component;
        let mut designator = String::new();
        let mut value = String::new();
        let mut description = String::new();
        let mut fields = HashMap::new();
        let mut lib_source = (LibName(String::new()), LibPartName(String::new()));
        for entry in component_entry {
            match entry {
                ComponentEntry::Ref(d) => designator = d.unwrap_or_default(),
                ComponentEntry::Value(v) => value = v.unwrap_or_default(),
                ComponentEntry::Footprint(f) => {
                    let f = f.unwrap_or_default();
                    if !f.is_empty() {
                        fields.insert("Footprint".into(), f);
                    }
                }
                ComponentEntry::Fields(f) => {
                    for field in f {
                        let value = field.1.unwrap_or_default();
                        if !value.is_empty() {
                            fields.insert(field.0, value);
                        }
                    }
                }
                ComponentEntry::LibSource {
                    lib,
                    part,
                    description: _,
                } => {
                    lib_source = (
                        LibName(lib.unwrap_or_default()),
                        LibPartName(part.unwrap_or_default()),
                    );
                }
                ComponentEntry::Property { name, value } => {
                    let name = name.unwrap_or_default();
                    let value = value.unwrap_or_default();
                    if !name.is_empty() && !value.is_empty() {
                        if name != "Sheetname" && name != "Sheetfile" {
                            fields.insert(name, value);
                        }
                    }
                }
                ComponentEntry::SheetPath { .. } => {}
                ComponentEntry::Tstamps(_) => {}
                ComponentEntry::Datasheet(d) => {
                    let d = d.unwrap_or_default();
                    if !d.is_empty() {
                        fields.insert("Datasheet".into(), d);
                    }
                }
                ComponentEntry::Description(d) => description = d.unwrap_or_default(),
            }
        }
        if designator.is_empty() {
            // TODO: kicad: emit warning on empty designator?
            continue;
        }
        components.insert(
            Designator(designator),
            Component {
                value,
                description,
                lib_source,
                fields,
                sections: vec![],
            },
        );
    }

    let mut lib_parts = HashMap::new();
    for part in netlist.libparts {
        let mut lib_source = (LibName(String::new()), LibPartName(String::new()));
        let mut description = String::new();
        let mut pins = HashMap::new();
        let mut fields = HashMap::new();
        let mut footprints = vec![];
        let LibPartKind::LibPart(lib_part_pieces) = part;
        for piece in lib_part_pieces {
            match piece {
                LibPartPiece::Lib(lib) => lib_source.0 = LibName(lib.unwrap_or_default()),
                LibPartPiece::Part(part) => lib_source.1 = LibPartName(part.unwrap_or_default()),
                LibPartPiece::Description(d) => description = d.unwrap_or_default(),
                LibPartPiece::Docs(_) => {}
                LibPartPiece::Footprints(f) => {
                    for footprint_kind in f {
                        let FootprintKind::Footprint(footprint_name) = footprint_kind;
                        let footprint_name = footprint_name.unwrap_or_default();
                        if !footprint_name.is_empty() {
                            footprints.push(footprint_name);
                        }
                    }
                }
                LibPartPiece::Fields(f) => {
                    for field in f {
                        let value = field.1.unwrap_or_default();
                        if !value.is_empty() {
                            fields.insert(field.0, value);
                        }
                    }
                }
                LibPartPiece::Pins(pin_kinds) => {
                    for p in pin_kinds {
                        let PinKind::Pin { num, name, r#type } = p;
                        pins.insert(
                            PinId(num.unwrap_or_default()),
                            Pin {
                                name: PinName(name.unwrap_or_default().to_uppercase()),
                                default_mode: PinMode {
                                    ty: PinType::DigitalInput, // TODO: Parse pin type
                                    pull_up: None,
                                    pull_down: None,
                                    io_standard: None,
                                },
                                alternate_modes: Default::default(),
                                bank_name: None,
                                section_name: None,
                            },
                        );
                    }
                }
            }
        }
        lib_parts.insert(
            lib_source,
            LibPart {
                description,
                footprints,
                fields,
                pins,
                banks: HashMap::new(),
            },
        );
    }

    let mut nets = HashMap::new();
    for net_kind in netlist.nets {
        let NetKind::Net(net_pieces) = net_kind;
        let mut net_name = String::new();
        let mut net = Net {
            nodes: HashSet::new(),
            properties: HashMap::new(),
        };
        for net_piece in net_pieces {
            match net_piece {
                NetPieceKind::Code(_) => {}
                NetPieceKind::Name(name) => {
                    net_name = name.unwrap_or_default();
                }
                NetPieceKind::Node {
                    r#ref,
                    pin,
                    pinfunction: _,
                    pintype: _,
                } => {
                    // TODO: emit warning if empty ref or pin
                    net.nodes.insert(Node {
                        designator: Designator(r#ref.unwrap_or_default()),
                        pin_id: PinId(pin.unwrap_or_default()),
                    });
                }
            }
        }
        // TODO: emit warnings
        if net_name.is_empty() {
            continue;
        }
        let net_name = NetName(net_name);
        if nets.contains_key(&net_name) {
            continue;
        }
        nets.insert(net_name, net);
    }
    Ok(Netlist {
        lib_parts,
        nets,
        components,
    })
}

#[derive(Debug, Serialize, Deserialize)]
enum KicadFileKind {
    #[serde(rename = "export")]
    NetListExport(KicadNetListExport),
}

#[derive(Debug, Serialize, Deserialize)]
struct KicadNetListExport {
    version: Option<String>,
    design: Vec<DesignPiece>,
    components: Vec<ComponentKind>,
    libparts: Vec<LibPartKind>,
    libraries: Vec<LibraryKind>,
    nets: Vec<NetKind>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DesignPiece {
    Source(Option<String>),
    Date(Option<String>),
    Tool(Option<String>),
    Sheet(Sheet),
}

#[derive(Debug, Serialize, Deserialize)]
struct Sheet {
    number: Option<String>,
    name: Option<String>,
    tstamps: Vec<String>,
    title_block: Vec<TitleBlockEntry>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum TitleBlockEntry {
    Title(Option<String>),
    Company(Option<String>),
    Rev(Option<String>),
    Date(Option<String>),
    Source(Option<String>),
    Comment {
        number: Option<String>,
        value: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum ComponentKind {
    #[serde(rename = "comp")]
    Component(Vec<ComponentEntry>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ComponentEntry {
    Ref(Option<String>),
    Value(Option<String>),
    Footprint(Option<String>),
    Fields(Vec<ComponentField>),
    #[serde(rename = "libsource")]
    LibSource {
        lib: Option<String>,
        part: Option<String>,
        description: Option<String>,
    },
    Property {
        name: Option<String>,
        value: Option<String>,
    },
    #[serde(rename = "sheetpath")]
    SheetPath {
        name: Option<String>,
        tstamps: Vec<String>,
    },
    Tstamps(Vec<String>),
    Datasheet(Option<String>),
    Description(Option<String>),
}

#[derive(Debug)]
struct ComponentField(String, Option<String>);

struct ComponentFieldKindVisitor;

impl<'de> serde::de::Visitor<'de> for ComponentFieldKindVisitor {
    type Value = ComponentField;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "'field (name \"param name\") \"value\"' OR 'field (name \"param name\")'"
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let _ = seq.next_element::<ComponentFieldPlainSymbol>()?.unwrap();
        let name = seq.next_element::<ComponentFieldName>()?.unwrap();
        let value = seq.next_element::<String>()?;
        let ComponentFieldName::Name(name) = name;
        Ok(ComponentField(name.unwrap(), value))
    }
}

impl<'de> Deserialize<'de> for ComponentField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ComponentFieldKindVisitor)
    }
}

impl Serialize for ComponentField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = if self.1.is_some() { 3 } else { 2 };
        let mut seq = serializer.serialize_seq(Some(len))?;
        seq.serialize_element(&ComponentFieldPlainSymbol::Field)?;
        seq.serialize_element(&ComponentFieldName::Name(Some(self.0.clone())))?;
        if let Some(value) = &self.1 {
            seq.serialize_element(value)?;
        }
        seq.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ComponentFieldPlainSymbol {
    Field,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ComponentFieldName {
    Name(Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
enum LibPartKind {
    #[serde(rename = "libpart")]
    LibPart(Vec<LibPartPiece>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LibPartPiece {
    Lib(Option<String>),
    Part(Option<String>),
    Description(Option<String>),
    Docs(Option<String>),
    Footprints(Vec<FootprintKind>),
    Fields(Vec<ComponentField>),
    Pins(Vec<PinKind>),
}

#[derive(Debug, Serialize, Deserialize)]
enum FootprintKind {
    #[serde(rename = "fp")]
    Footprint(Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
enum PinKind {
    #[serde(rename = "pin")]
    Pin {
        num: Option<String>,
        name: Option<String>,
        r#type: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum LibraryKind {
    #[serde(rename = "library")]
    Library {
        logical: Option<String>,
        uri: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum NetKind {
    #[serde(rename = "net")]
    Net(Vec<NetPieceKind>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum NetPieceKind {
    Code(Option<String>),
    Name(Option<String>),
    Node {
        r#ref: Option<String>,
        pin: Option<String>,
        pinfunction: Option<String>,
        pintype: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::{DesignPiece, KicadFileKind, load_kicad_netlist};
    use crate::netlist::{Net, Node};
    use crate::{Designator, NetName, PinId};
    use std::fs::read_to_string;
    use std::path::PathBuf;

    #[test]
    fn can_read_netlist_kicad_sexpr() {
        let contents = read_to_string("test_input/netlist_kicad.net").unwrap();
        let netlist: KicadFileKind = serde_lexpr::from_str(&contents).unwrap();
        let KicadFileKind::NetListExport(netlist) = netlist;
        assert_eq!(netlist.design.len(), 5);
        assert_eq!(netlist.components.len(), 3);
        assert!(matches!(&netlist.design[2], DesignPiece::Tool(_)));
        if let DesignPiece::Tool(tool) = &netlist.design[2] {
            assert_eq!(tool, &Some("Eeschema 8.0.4".into()));
        }
    }

    #[test]
    fn can_read_netlist_kicad() {
        let netlist = load_kicad_netlist(&PathBuf::from("test_input/netlist_kicad.net")).unwrap();
        assert_eq!(netlist.nets.len(), 4);
        assert_eq!(
            netlist.nets.get(&NetName("/Eth/RXD0".into())),
            Some(&Net {
                nodes: [
                    Node {
                        designator: Designator("R21".to_string()),
                        pin_id: PinId("2".to_string())
                    },
                    Node {
                        designator: Designator("R29".to_string()),
                        pin_id: PinId("2".to_string())
                    },
                    Node {
                        designator: Designator("U2".to_string()),
                        pin_id: PinId("11".to_string())
                    }
                ]
                .into(),
                properties: Default::default(),
            })
        )
    }
}
