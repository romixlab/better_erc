//! Bus names as KiCad reads them: vectors (`D[0..7]`, members `D0`..`D7`) and groups (`USB{DP DM}`, members
//! `USB.DP`, `USB.DM`; members may be vectors themselves, `{PU_EN[0..7],PD_EN[0..7]}`). Formatting braces
//! (`~{RST}`, `^{2}`, `_{n}`) are part of a name, not a group.

/// A bus as its label names it
#[derive(Clone, Debug, PartialEq)]
pub enum Bus {
    /// `PREFIX[a..b]SUFFIX`: members by index
    Vector {
        prefix: String,
        members: Vec<String>,
    },
    /// `PREFIX{a b c}`: members by name; a member can be a vector
    Group {
        prefix: String,
        members: Vec<GroupMember>,
    },
}

/// A member of a bus group
#[derive(Clone, Debug, PartialEq)]
pub enum GroupMember {
    Net(String),
    Vector(Vec<String>),
}

fn is_format(c: char) -> bool {
    matches!(c, '~' | '^' | '_')
}

impl Bus {
    /// The bus a label text names, `None` for a plain net name
    pub fn parse(text: &str) -> Option<Bus> {
        vector(text).or_else(|| group(text))
    }

    /// Member net names, flattened, without the sheet path: `D3`, `USB.DP`, `PU_EN3`
    pub fn member_names(&self) -> Vec<String> {
        match self {
            Bus::Vector { members, .. } => members.clone(),
            Bus::Group { prefix, members } => {
                let p = if prefix.is_empty() {
                    String::new()
                } else {
                    format!("{prefix}.")
                };
                members
                    .iter()
                    .flat_map(|m| match m {
                        GroupMember::Net(n) => vec![format!("{p}{n}")],
                        GroupMember::Vector(v) => v.iter().map(|n| format!("{p}{n}")).collect(),
                    })
                    .collect()
            }
        }
    }

    /// The keys members are matched by between two buses: the index for a vector, the name inside the group
    /// (without the group's prefix) for a group
    pub fn member_keys(&self) -> Vec<String> {
        match self {
            Bus::Vector { members, .. } => (0..members.len()).map(|i| format!("#{i}")).collect(),
            Bus::Group { members, .. } => members
                .iter()
                .flat_map(|m| match m {
                    GroupMember::Net(n) => vec![n.clone()],
                    GroupMember::Vector(v) => v.clone(),
                })
                .collect(),
        }
    }
}

/// `PREFIX[a..b]SUFFIX`
fn vector(text: &str) -> Option<Bus> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut prefix = String::new();
    let mut depth = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if i > 0 && is_format(chars[i - 1]) {
                depth += 1;
                prefix.push(c);
                i += 1;
                continue;
            }
            return None;
        }
        if c == '}' {
            depth -= 1;
            prefix.push(c);
            i += 1;
            continue;
        }
        if c == ' ' || c == ']' {
            return None;
        }
        if c == '[' {
            break;
        }
        prefix.push(c);
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    // Formatting that wraps the range only (`D_{[1..2]}`) is dropped from the prefix
    let mut wraps_name = false;
    if depth > 0
        && let Some(open) = prefix.rfind('{')
        && open > 0
        && prefix[..open].ends_with(is_format)
    {
        if open == prefix.len() - 1 {
            prefix.truncate(open - 1);
        } else {
            wraps_name = true;
        }
    }
    i += 1;
    let mut begin = String::new();
    while i < chars.len() {
        if chars[i] == '.' && chars.get(i + 1) == Some(&'.') {
            i += 2;
            break;
        }
        if !chars[i].is_ascii_digit() {
            return None;
        }
        begin.push(chars[i]);
        i += 1;
    }
    let mut end = String::new();
    let mut closed = false;
    while i < chars.len() {
        if chars[i] == ']' {
            i += 1;
            closed = true;
            break;
        }
        if !chars[i].is_ascii_digit() {
            return None;
        }
        end.push(chars[i]);
        i += 1;
    }
    if !closed {
        return None;
    }
    let mut suffix = String::new();
    while i < chars.len() {
        match chars[i] {
            '}' => {
                depth -= 1;
                if wraps_name {
                    suffix.push('}');
                }
            }
            c @ ('+' | '-' | 'P' | 'N') => suffix.push(c),
            _ => return None,
        }
        i += 1;
    }
    if depth != 0 {
        return None;
    }
    let (mut a, mut b): (i64, i64) = (begin.parse().ok()?, end.parse().ok()?);
    if a == b {
        return None;
    }
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    let members = (a..=b).map(|n| format!("{prefix}{n}{suffix}")).collect();
    Some(Bus::Vector { prefix, members })
}

/// `PREFIX{a b,c}`
fn group(text: &str) -> Option<Bus> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut prefix = String::new();
    let mut depth = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if i > 0 && is_format(chars[i - 1]) {
                depth += 1;
                prefix.push(c);
                i += 1;
                continue;
            }
            break;
        }
        if c == '}' {
            depth -= 1;
            prefix.push(c);
            i += 1;
            continue;
        }
        if c == ' ' || c == '[' || c == ']' {
            return None;
        }
        prefix.push(c);
        i += 1;
    }
    if depth != 0 || i >= chars.len() {
        return None;
    }
    i += 1;
    let mut members = Vec::new();
    let mut tmp = String::new();
    let push = |tmp: &mut String, members: &mut Vec<GroupMember>| {
        if !tmp.is_empty() {
            let name = tmp.replace('/', "{slash}");
            members.push(match vector(&name) {
                Some(Bus::Vector { members, .. }) => GroupMember::Vector(members),
                _ => GroupMember::Net(name),
            });
            tmp.clear();
        }
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if i > 0 && is_format(chars[i - 1]) {
                depth += 1;
                tmp.push(c);
                i += 1;
                continue;
            }
            return None;
        }
        if c == '}' {
            if depth > 0 {
                depth -= 1;
                tmp.push(c);
                i += 1;
                continue;
            }
            push(&mut tmp, &mut members);
            return Some(Bus::Group { prefix, members });
        }
        if c == ' ' || c == ',' {
            push(&mut tmp, &mut members);
        } else {
            tmp.push(c);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectors() {
        let b = Bus::parse("D[0..3]").unwrap();
        assert_eq!(b.member_names(), vec!["D0", "D1", "D2", "D3"]);
        assert_eq!(
            Bus::parse("A[3..1]").unwrap().member_names(),
            vec!["A1", "A2", "A3"]
        );
        assert_eq!(
            Bus::parse("~{BE[0..1]}").unwrap().member_names(),
            vec!["~{BE0}", "~{BE1}"]
        );
        assert_eq!(
            Bus::parse("D_{[1..2]}").unwrap().member_names(),
            vec!["D1", "D2"]
        );
        assert_eq!(
            Bus::parse("LVDS[0..1]P").unwrap().member_names(),
            vec!["LVDS0P", "LVDS1P"]
        );
        assert_eq!(Bus::parse("~{RESET}"), None);
        assert_eq!(Bus::parse("SDA"), None);
        assert_eq!(Bus::parse("X[1..1]"), None);
    }

    #[test]
    fn groups() {
        let b = Bus::parse("EXP{CLK LATCH ~{OE}}").unwrap();
        assert_eq!(b.member_names(), vec!["EXP.CLK", "EXP.LATCH", "EXP.~{OE}"]);
        assert_eq!(b.member_keys(), vec!["CLK", "LATCH", "~{OE}"]);
        let b = Bus::parse("{PU_EN[0..1],PD_EN[0..1]}").unwrap();
        assert_eq!(
            b.member_names(),
            vec!["PU_EN0", "PU_EN1", "PD_EN0", "PD_EN1"]
        );
        assert_eq!(Bus::parse("~{RST}"), None);
        assert_eq!(Bus::parse("INA_SPI.~{ALERT}"), None);
    }
}
