use std::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{SeqAccess};
use serde::ser::SerializeSeq;

#[derive(Debug, Serialize, Deserialize)]
pub enum KicadFileKind {
    #[serde(rename = "export")]
    NetListExport(KicadNetListExport)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KicadNetListExport {
    pub version: Option<String>,
    pub design: Vec<DesignPiece>,
    pub components: Vec<ComponentKind>,
    pub libparts: Vec<LibPartKind>,
    pub libraries: Vec<LibraryKind>,
    pub nets: Vec<NetKind>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesignPiece {
    Source(Option<String>),
    Date(Option<String>),
    Tool(Option<String>),
    Sheet(Sheet),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sheet {
    pub number: Option<String>,
    pub name: Option<String>,
    pub tstamps: Vec<String>,
    pub title_block: Vec<TitleBlockEntry>
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TitleBlockEntry {
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
pub enum ComponentKind {
    #[serde(rename = "comp")]
    Component(Vec<ComponentEntry>)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentEntry {
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
pub struct ComponentField(String, Option<String>);

struct ComponentFieldKindVisitor;

impl<'de> serde::de::Visitor<'de> for ComponentFieldKindVisitor {
    type Value = ComponentField;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "'field (name \"param name\") \"value\"' OR 'field (name \"param name\")'")
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
        D: Deserializer<'de>
    {
        deserializer.deserialize_seq(ComponentFieldKindVisitor)
    }
}

impl Serialize for ComponentField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
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
pub enum ComponentFieldPlainSymbol {
    Field,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentFieldName {
    Name(Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LibPartKind {
    #[serde(rename = "libpart")]
    LibPart(Vec<LibPartPiece>)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibPartPiece {
    Lib(Option<String>),
    Part(Option<String>),
    Description(Option<String>),
    Docs(Option<String>),
    Footprints(Vec<FootprintKind>),
    Fields(Vec<ComponentField>),
    Pins(Vec<PinKind>),
}


#[derive(Debug, Serialize, Deserialize)]
pub enum FootprintKind {
    #[serde(rename = "fp")]
    Footprint(Option<String>)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PinKind {
    #[serde(rename = "pin")]
    Pin {
        num: Option<String>,
        name: Option<String>,
        r#type: Option<String>,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LibraryKind {
    #[serde(rename = "library")]
    Library {
        logical: Option<String>,
        uri: Option<String>,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NetKind {
    #[serde(rename = "net")]
    Net(Vec<NetPieceKind>)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetPieceKind {
    Code(Option<String>),
    Name(Option<String>),
    Node {
        r#ref: Option<String>,
        pin: Option<String>,
        pinfunction: Option<String>,
        pintype: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;
    use super::{DesignPiece, KicadFileKind};

    #[test]
    fn can_read_netlist_kicad() {
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
}