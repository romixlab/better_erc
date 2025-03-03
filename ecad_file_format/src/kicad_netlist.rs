use crate::netlist::{Net, Netlist, Node};
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

    let parts = HashMap::new();
    let mut nets = HashMap::new();
    for net_kind in netlist.nets {
        let NetKind::Net(net_pieces) = net_kind;
        let mut net_name = String::new();
        let mut net = Net {
            nodes: HashSet::new(),
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
                        part_ref: r#ref.unwrap_or_default(),
                        part_pin: pin.unwrap_or_default(),
                    });
                }
            }
        }
        // TODO: emit warnings
        if net_name.is_empty() {
            continue;
        }
        if nets.contains_key(&net_name) {
            continue;
        }
        nets.insert(net_name, net);
    }
    Ok(Netlist { parts, nets })
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
            netlist.nets.get("/Eth/RXD0"),
            Some(&Net {
                nodes: [
                    Node {
                        part_ref: "R21".to_string(),
                        part_pin: "2".to_string()
                    },
                    Node {
                        part_ref: "R29".to_string(),
                        part_pin: "2".to_string()
                    },
                    Node {
                        part_ref: "U2".to_string(),
                        part_pin: "11".to_string()
                    }
                ]
                .into(),
            })
        )
    }
}
