//! erc_parts: the board's MPNs, the lock's text, a refresh against a fake source

mod board;
mod lock;
mod refresh;

use erc_model::{Design, ImportOptions, MemFiles};

/// Field names and values
pub type Fields<'a> = &'a [(&'a str, &'a str)];

/// One resistor: its reference, its fields beyond the usual, whether it is DNP, and KiCad 10 variant blocks
pub struct Part<'a> {
    pub reference: &'a str,
    pub fields: Fields<'a>,
    pub dnp: bool,
    /// `(variant, dnp, fields)`
    pub variants: &'a [(&'a str, bool, Fields<'a>)],
}

/// A KiCad 10 schematic of these parts
pub fn schematic(parts: &[Part]) -> String {
    let mut s = String::from(
        "(kicad_sch (version 20260306) (generator \"eeschema\") (generator_version \"10.0\")\n\
         \t(uuid \"00000000-0000-4000-8000-000000000000\")\n\t(paper \"A4\")\n\t(lib_symbols\n\
         \t\t(symbol \"Fixture:R\" (exclude_from_sim no) (in_bom yes) (on_board yes)\n\
         \t\t\t(property \"Reference\" \"R\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n\
         \t\t\t(property \"Value\" \"R\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n\
         \t\t\t(symbol \"R_1_1\"\n\
         \t\t\t\t(pin passive line (at 0 3.81 270) (length 1.27) (name \"~\" (effects (font (size 1.27 1.27)))) (number \"1\" (effects (font (size 1.27 1.27)))))\n\
         \t\t\t\t(pin passive line (at 0 -3.81 90) (length 1.27) (name \"~\" (effects (font (size 1.27 1.27)))) (number \"2\" (effects (font (size 1.27 1.27)))))\n\
         \t\t\t)\n\t\t)\n\t)\n",
    );
    for (i, p) in parts.iter().enumerate() {
        let x = 30 + 20 * i;
        let dnp = if p.dnp { "yes" } else { "no" };
        s += &format!(
            "\t(symbol (lib_id \"Fixture:R\") (at {x} 40 0) (unit 1) (exclude_from_sim no) (in_bom yes) (on_board yes) (dnp {dnp}) (uuid \"00000000-0000-4000-8000-{:012}\")\n",
            i + 1
        );
        let mut fields = vec![("Reference", p.reference), ("Value", "10k")];
        fields.extend(p.fields.iter().copied());
        for (name, value) in fields {
            s += &format!(
                "\t\t(property \"{name}\" \"{value}\" (at {x} 40 0) (effects (font (size 1.27 1.27))))\n"
            );
        }
        s += &format!(
            "\t\t(instances (project \"\" (path \"/00000000-0000-4000-8000-000000000000\" (reference \"{}\") (unit 1)",
            p.reference
        );
        for (name, dnp, fields) in p.variants {
            let dnp = if *dnp { "yes" } else { "no" };
            s += &format!(" (variant (name \"{name}\") (dnp {dnp})");
            for (f, v) in *fields {
                s += &format!(" (field (name \"{f}\") (value \"{v}\"))");
            }
            s += ")";
        }
        s += ")))\n\t)\n";
    }
    s += ")\n";
    s
}

/// The design of a schematic's text
pub fn design(text: &str) -> Design {
    let files = MemFiles::default().with("sheet.kicad_sch", text);
    erc_kicad::project::read(&files, "sheet.kicad_sch", &ImportOptions::default())
        .expect("the test schematic reads")
        .design
}
