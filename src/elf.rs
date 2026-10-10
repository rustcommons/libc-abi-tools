//! Bounded ELF64 x86_64 dynamic-symbol evidence. No alias semantics are inferred.
//!
//! Layout follows the System V ABI and GNU version-definition extensions.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::File,
    io::Read,
    path::Path,
};

pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const DYNSYM: u32 = 11;
const STRTAB: u32 = 3;
const DYNAMIC: u32 = 6;
const VERDEF: u32 = 0x6fff_fffd;
const VERSYM: u32 = 0x6fff_ffff;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Symbol {
    pub name: String,
    pub version: Option<String>,
    /// None for unversioned symbols; false for GNU nondefault versions.
    pub default_version: Option<bool>,
    pub symbol_type: u8,
    pub binding: u8,
    pub visibility: u8,
    pub section: u16,
    /// Declared alignment of the defining section; zero for special indexes.
    pub section_alignment: u64,
    pub value: u64,
    pub size: u64,
    /// Same section/value/type/size in this file, not a proven semantic alias.
    pub same_range: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    pub soname: String,
    pub symbols: Vec<Symbol>,
}

#[derive(Clone, Copy)]
struct Section {
    kind: u32,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    entry_size: u64,
    alignment: u64,
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn slice(&self, offset: u64, size: u64) -> Result<&'a [u8], String> {
        let end = offset.checked_add(size).ok_or("ELF range overflow")?;
        let start = usize::try_from(offset).map_err(|_| "ELF offset too large")?;
        let end = usize::try_from(end).map_err(|_| "ELF range too large")?;
        self.0
            .get(start..end)
            .ok_or_else(|| "ELF range outside input".into())
    }
    fn u16(&self, offset: u64) -> Result<u16, String> {
        let mut bytes = [0; 2];
        bytes.copy_from_slice(self.slice(offset, 2)?);
        Ok(u16::from_le_bytes(bytes))
    }
    fn u32(&self, offset: u64) -> Result<u32, String> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.slice(offset, 4)?);
        Ok(u32::from_le_bytes(bytes))
    }
    fn u64(&self, offset: u64) -> Result<u64, String> {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.slice(offset, 8)?);
        Ok(u64::from_le_bytes(bytes))
    }
    fn string(&self, offset: u32) -> Result<String, String> {
        let tail = self
            .0
            .get(offset as usize..)
            .ok_or("ELF string offset outside table")?;
        let end = tail
            .iter()
            .position(|&byte| byte == 0)
            .ok_or("unterminated ELF string")?;
        let bytes = &tail[..end];
        if bytes.is_empty() || bytes.len() > 1024 || !bytes.iter().all(u8::is_ascii_graphic) {
            return Err("ELF names must be nonempty printable ASCII, at most 1024 bytes".into());
        }
        String::from_utf8(bytes.to_vec()).map_err(|_| "invalid ELF string".into())
    }
}

fn one(sections: &[Section], kind: u32) -> Result<Option<(usize, Section)>, String> {
    let mut found = sections
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, s)| s.kind == kind);
    let first = found.next();
    if found.next().is_some() {
        return Err(format!("multiple ELF sections of type {kind:#x}"));
    }
    Ok(first)
}

fn strings<'a>(r: &Reader<'a>, sections: &[Section], link: u32) -> Result<Reader<'a>, String> {
    let section = sections
        .get(link as usize)
        .ok_or("invalid ELF string-table link")?;
    if section.kind != STRTAB {
        return Err("ELF link does not identify a string table".into());
    }
    let data = r.slice(section.offset, section.size)?;
    if data.first() != Some(&0) {
        return Err("ELF string table lacks initial NUL".into());
    }
    Ok(Reader(data))
}

fn versions(
    r: &Reader<'_>,
    section: Section,
    names: &Reader<'_>,
) -> Result<BTreeMap<u16, String>, String> {
    let data = Reader(r.slice(section.offset, section.size)?);
    if section.info == 0 || section.info > 4096 || u64::from(section.info) > section.size / 20 {
        return Err("invalid ELF version-definition count".into());
    }
    let mut result = BTreeMap::new();
    let mut offset = 0;
    let mut name_bytes = 0;
    for index in 0..section.info {
        data.slice(offset, 20)?;
        if data.u16(offset)? != 1 {
            return Err("unsupported ELF version-definition revision".into());
        }
        let id = data.u16(offset + 4)?;
        let count = data.u16(offset + 6)?;
        let aux = u64::from(data.u32(offset + 12)?);
        let next = u64::from(data.u32(offset + 16)?);
        if id == 0 || id >= 0x8000 || count == 0 || count > 256 || aux < 20 {
            return Err("invalid ELF version definition".into());
        }
        let mut auxiliary = offset + aux;
        let mut version_name = None;
        for item in 0..count {
            data.slice(auxiliary, 8)?;
            let name = names.string(data.u32(auxiliary)?)?;
            name_bytes += name.len();
            if name_bytes > 1024 * 1024 {
                return Err("ELF version-name budget exceeded".into());
            }
            if item == 0 {
                version_name = Some(name);
            }
            let advance = u64::from(data.u32(auxiliary + 4)?);
            if item + 1 < count {
                if advance < 8 {
                    return Err("invalid ELF version auxiliary chain".into());
                }
                auxiliary += advance;
            } else if advance != 0 {
                return Err("unterminated ELF version auxiliary chain".into());
            }
        }
        let name = version_name.ok_or("missing ELF version name")?;
        if result.insert(id, name).is_some() {
            return Err("duplicate ELF version index".into());
        }
        if index + 1 < section.info {
            if next < 20 || offset + next < auxiliary + 8 {
                return Err("invalid ELF version-definition chain".into());
            }
            offset += next;
        } else if next != 0 {
            return Err("unterminated ELF version-definition chain".into());
        }
    }
    Ok(result)
}

impl Metadata {
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut bytes = Vec::new();
        file.take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("ELF exceeds 64 MiB input limit".into());
        }
        let r = Reader(bytes);
        let header = r.slice(0, 64)?;
        if &header[..7] != b"\x7fELF\x02\x01\x01" || !matches!(header[7], 0 | 3) || header[8] != 0 {
            return Err("expected ELF64 little-endian System V/Linux input".into());
        }
        if r.u16(16)? != 3 || r.u16(18)? != 62 || r.u32(20)? != 1 || r.u16(52)? != 64 {
            return Err("expected an x86_64 ELF shared object".into());
        }
        let offset = r.u64(40)?;
        let count = r.u16(60)?;
        if offset == 0 || count == 0 || r.u16(58)? != 64 {
            return Err(
                "ELF requires ordinary 64-byte section headers; extended/absent tables unsupported"
                    .into(),
            );
        }
        r.slice(offset, u64::from(count) * 64)?;
        let mut sections = Vec::new();
        for index in 0..count {
            let pos = offset + u64::from(index) * 64;
            sections.push(Section {
                kind: r.u32(pos + 4)?,
                offset: r.u64(pos + 24)?,
                size: r.u64(pos + 32)?,
                link: r.u32(pos + 40)?,
                info: r.u32(pos + 44)?,
                entry_size: r.u64(pos + 56)?,
                alignment: r.u64(pos + 48)?,
            });
        }
        let (sym_index, sym_section) =
            one(&sections, DYNSYM)?.ok_or("missing ELF dynamic symbols")?;
        if sym_section.entry_size != 24 || sym_section.size == 0 || sym_section.size % 24 != 0 {
            return Err("invalid ELF dynamic-symbol entry size".into());
        }
        let names = strings(&r, &sections, sym_section.link)?;
        let symbols = Reader(r.slice(sym_section.offset, sym_section.size)?);
        if symbols.slice(0, 24)?.iter().any(|&byte| byte != 0) {
            return Err("ELF dynamic-symbol zero entry is not null".into());
        }
        let (_, dynamic) = one(&sections, DYNAMIC)?.ok_or("missing ELF dynamic section")?;
        if dynamic.entry_size != 16 || dynamic.size % 16 != 0 {
            return Err("invalid ELF dynamic entry size".into());
        }
        let dynamic_names = strings(&r, &sections, dynamic.link)?;
        let dynamic_data = Reader(r.slice(dynamic.offset, dynamic.size)?);
        let mut soname = None;
        let mut terminated = false;
        for pos in (0..dynamic.size).step_by(16) {
            let tag = dynamic_data.u64(pos)?;
            if tag == 0 {
                terminated = true;
                break;
            }
            if tag == 14 {
                let name = u32::try_from(dynamic_data.u64(pos + 8)?)
                    .map_err(|_| "ELF SONAME offset too large")?;
                if soname.replace(dynamic_names.string(name)?).is_some() {
                    return Err("duplicate ELF SONAME".into());
                }
            }
        }
        if !terminated {
            return Err("unterminated ELF dynamic section".into());
        }
        let soname = soname.ok_or("ELF shared object lacks SONAME")?;
        let symbol_count = sym_section.size / 24;
        if symbol_count > 100_000 {
            return Err("ELF dynamic-symbol count exceeds 100000".into());
        }
        let version_section = one(&sections, VERSYM)?;
        let version_symbols = if let Some((_, section)) = version_section {
            if section.link as usize != sym_index
                || section.entry_size != 2
                || section.size != symbol_count * 2
            {
                return Err("invalid ELF version-symbol table".into());
            }
            Some(Reader(r.slice(section.offset, section.size)?))
        } else {
            None
        };
        let version_names = if let Some((_, section)) = one(&sections, VERDEF)? {
            if version_symbols.is_none() || section.link != sym_section.link {
                return Err("invalid ELF version-definition link".into());
            }
            versions(&r, section, &names)?
        } else {
            BTreeMap::new()
        };
        let mut entries = Vec::new();
        let mut keys = BTreeSet::new();
        let mut defaults = BTreeSet::new();
        let mut name_bytes = 0;
        for index in 1..symbol_count {
            let pos = index * 24;
            let info = symbols.slice(pos + 4, 1)?[0];
            let binding = info >> 4;
            let section = symbols.u16(pos + 6)?;
            // Imported or local symbols are not definitions supplied by this DSO.
            if section == 0 || binding == 0 {
                continue;
            }
            if !matches!(binding, 1 | 2 | 10) {
                return Err(format!("unsupported ELF symbol binding {binding}"));
            }
            if section == 0xffff
                || (section as usize >= sections.len() && !matches!(section, 0xfff1 | 0xfff2))
            {
                return Err("unsupported or invalid ELF symbol section index".into());
            }
            let visibility = symbols.slice(pos + 5, 1)?[0];
            let section_alignment = sections
                .get(section as usize)
                .map_or(0, |section| section.alignment);
            if section_alignment != 0 && !section_alignment.is_power_of_two() {
                return Err("invalid ELF section alignment".into());
            }
            if visibility > 3 {
                return Err("unsupported ELF symbol visibility bits".into());
            }
            let name = names.string(symbols.u32(pos)?)?;
            let raw = match &version_symbols {
                Some(table) => table.u16(index * 2)?,
                None => 1,
            };
            let id = raw & 0x7fff;
            if section == 0xfff1 && version_names.values().any(|version| version == &name) {
                continue;
            }
            let (version, default_version) = if id > 1 {
                (
                    Some(
                        version_names
                            .get(&id)
                            .ok_or("undefined ELF version-definition index")?
                            .clone(),
                    ),
                    Some(raw & 0x8000 == 0),
                )
            } else {
                if raw & 0x8000 != 0 || id == 0 {
                    return Err("invalid global ELF symbol version index".into());
                }
                (None, None)
            };
            name_bytes += name.len() + version.as_ref().map_or(0, String::len);
            if name_bytes > 8 * 1024 * 1024 {
                return Err("ELF symbol-name budget exceeded".into());
            }
            if !keys.insert((name.clone(), version.clone(), default_version)) {
                return Err("duplicate ELF symbol definition".into());
            }
            if default_version == Some(true) && !defaults.insert(name.clone()) {
                return Err("multiple default ELF symbol versions".into());
            }
            entries.push(Symbol {
                name,
                version,
                default_version,
                symbol_type: info & 15,
                binding,
                visibility,
                section,
                section_alignment,
                value: symbols.u64(pos + 8)?,
                size: symbols.u64(pos + 16)?,
                same_range: None,
            });
        }
        entries.sort();
        let mut ranges = BTreeMap::<_, Vec<usize>>::new();
        for (index, symbol) in entries.iter().enumerate() {
            if symbol.section < 0xff00 && symbol.size != 0 {
                ranges
                    .entry((
                        symbol.section,
                        symbol.value,
                        symbol.symbol_type,
                        symbol.size,
                    ))
                    .or_default()
                    .push(index);
            }
        }
        let mut group = 0;
        for indexes in ranges.values().filter(|indexes| indexes.len() > 1) {
            group += 1;
            for &index in indexes {
                entries[index].same_range = Some(group);
            }
        }
        Ok(Self {
            soname,
            symbols: entries,
        })
    }

    pub fn to_tsv(&self) -> String {
        let mut text = String::from("soname\tsymbol\tversion\tdefault\ttype\tbinding\tvisibility\tsection\tsection_alignment\tvalue\tsize\tsame_range\n");
        for s in &self.symbols {
            let kind = match s.symbol_type {
                0 => "NOTYPE".into(),
                1 => "OBJECT".into(),
                2 => "FUNC".into(),
                6 => "TLS".into(),
                10 => "IFUNC".into(),
                n => format!("TYPE_{n}"),
            };
            let binding = match s.binding {
                1 => "GLOBAL",
                2 => "WEAK",
                10 => "GNU_UNIQUE",
                _ => "UNKNOWN",
            };
            let visibility = match s.visibility {
                0 => "DEFAULT",
                1 => "INTERNAL",
                2 => "HIDDEN",
                3 => "PROTECTED",
                _ => "UNKNOWN",
            };
            let default = match s.default_version {
                Some(true) => "yes",
                Some(false) => "no",
                None => "-",
            };
            let group = s
                .same_range
                .map_or_else(|| "-".into(), |value| value.to_string());
            writeln!(
                text,
                "{}\t{}\t{}\t{default}\t{kind}\t{binding}\t{visibility}\t{}\t{}\t0x{:x}\t{}\t{group}",
                self.soname,
                s.name,
                s.version.as_deref().unwrap_or("-"),
                s.section,
                s.section_alignment,
                s.value,
                s.size
            )
            .expect("writing to a String cannot fail");
        }
        text
    }
}
