//! A small, bounds-checked reader for x86_64 ELF64 little-endian executables
//! and shared objects: just what the scan needs. Malformed input returns an
//! error; nothing panics on untrusted bytes.

/// Largest input accepted.
pub const MAX_BYTES: usize = 512 * 1024 * 1024;

const EM_X86_64: u16 = 62;
const SHT_RELA: u32 = 4;
const SHT_DYNSYM: u32 = 11;
const SHT_NOTE: u32 = 7;
const SHT_GNU_VERNEED: u32 = 0x6fff_fffe;
const SHT_GNU_VERSYM: u32 = 0x6fff_ffff;
const SHF_EXECINSTR: u64 = 4;
const PT_DYNAMIC: u32 = 2;
const PT_NOTE: u32 = 4;
const VER_FLG_WEAK: u16 = 2;
const STB_WEAK: u8 = 2;
const DT_RELR: i64 = 36;
const NT_GNU_ABI_TAG: u32 = 1;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_JUMP_SLOT: u32 = 7;

/// An undefined (imported) dynamic symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    pub name: String,
    /// Symbol version, e.g. `GLIBC_2.28`, if versioned.
    pub version: Option<String>,
    /// The library the version is required from, e.g. `libc.so.6`.
    pub file: Option<String>,
    /// Weak symbol, or a weak version requirement: the program must cope
    /// with it being absent.
    pub weak: bool,
    /// GOT slots (virtual addresses) that hold this symbol's address.
    pub slots: Vec<u64>,
}

/// One version requirement (`.gnu.version_r`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionNeed {
    pub file: String,
    pub version: String,
    pub weak: bool,
}

/// An executable section: its virtual address and bytes.
#[derive(Clone, Debug)]
pub struct Code<'a> {
    pub name: String,
    pub addr: u64,
    pub bytes: &'a [u8],
}

/// What the scan reads from one ELF file.
#[derive(Debug)]
pub struct Elf<'a> {
    pub imports: Vec<Import>,
    pub needs: Vec<VersionNeed>,
    pub code: Vec<Code<'a>>,
    /// Minimum kernel from `NT_GNU_ABI_TAG` (Linux only), as `[major, minor, patch]`.
    pub abi_tag: Option<[u32; 3]>,
    /// `DT_RELR` relative relocations are present.
    pub relr: bool,
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn slice(&self, off: u64, len: u64) -> Result<&'a [u8], String> {
        let start = usize::try_from(off).map_err(|_| "offset out of range")?;
        let len = usize::try_from(len).map_err(|_| "length out of range")?;
        let end = start.checked_add(len).ok_or("range overflows")?;
        self.0
            .get(start..end)
            .ok_or_else(|| format!("range {start:#x}..{end:#x} is outside the file"))
    }
    fn u8(&self, off: u64) -> Result<u8, String> {
        Ok(self.slice(off, 1)?[0])
    }
    fn u16(&self, off: u64) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.slice(off, 2)?.try_into().unwrap()))
    }
    fn u32(&self, off: u64) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.slice(off, 4)?.try_into().unwrap()))
    }
    fn u64(&self, off: u64) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.slice(off, 8)?.try_into().unwrap()))
    }
    fn cstr(&self, table: &Section, off: u64) -> Result<String, String> {
        if off >= table.size {
            return Err("string offset outside its table".into());
        }
        let bytes = self.slice(table.offset + off, table.size - off)?;
        let end = bytes
            .iter()
            .position(|&b| b == 0)
            .ok_or("unterminated string")?;
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
}

#[derive(Clone, Debug)]
struct Section {
    name_off: u32,
    kind: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    entsize: u64,
}

impl<'a> Elf<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("file exceeds the 512 MiB scan limit".into());
        }
        let r = Reader(bytes);
        if r.slice(0, 4)? != b"\x7fELF" {
            return Err("not an ELF file".into());
        }
        if r.u8(4)? != 2 || r.u8(5)? != 1 {
            return Err("only ELF64 little-endian files are supported".into());
        }
        if r.u16(18)? != EM_X86_64 {
            return Err("only x86_64 ELF files are supported".into());
        }
        let (phoff, shoff) = (r.u64(32)?, r.u64(40)?);
        let (phentsize, phnum) = (r.u16(54)? as u64, r.u16(56)? as u64);
        let (shentsize, shnum, shstrndx) = (r.u16(58)? as u64, r.u16(60)? as u64, r.u16(62)?);
        if shoff == 0 || shnum == 0 {
            return Err("the file has no section headers (stripped with --strip-sections?); the scan needs them".into());
        }
        if shentsize != 64 || (phnum > 0 && phentsize != 56) {
            return Err("unexpected ELF header entry sizes".into());
        }
        let mut sections = Vec::new();
        for i in 0..shnum {
            let o = shoff + i * 64;
            sections.push(Section {
                name_off: r.u32(o)?,
                kind: r.u32(o + 4)?,
                flags: r.u64(o + 8)?,
                addr: r.u64(o + 16)?,
                offset: r.u64(o + 24)?,
                size: r.u64(o + 32)?,
                link: r.u32(o + 40)?,
                entsize: r.u64(o + 56)?,
            });
        }
        let shstr = sections
            .get(shstrndx as usize)
            .cloned()
            .ok_or("section name table index is out of range")?;
        let name = |s: &Section| r.cstr(&shstr, s.name_off as u64);

        // Dynamic symbols, their versions, and the version requirements.
        let mut imports = Vec::new();
        let mut needs = Vec::new();
        let dynsym_index = sections.iter().position(|s| s.kind == SHT_DYNSYM);
        let mut version_names: std::collections::BTreeMap<u16, (String, String, bool)> =
            Default::default();
        if let Some(vn) = sections
            .iter()
            .find(|s| s.kind == SHT_GNU_VERNEED && s.size > 0)
        {
            let strtab = sections.get(vn.link as usize).ok_or("bad verneed link")?;
            let mut off = vn.offset;
            for _ in 0..4096 {
                let cnt = r.u16(off + 2)?;
                let file = r.cstr(strtab, r.u32(off + 4)? as u64)?;
                let mut aux = off + r.u32(off + 8)? as u64;
                for _ in 0..cnt {
                    let flags = r.u16(aux + 4)?;
                    let index = r.u16(aux + 6)?;
                    let version = r.cstr(strtab, r.u32(aux + 8)? as u64)?;
                    let weak = flags & VER_FLG_WEAK != 0;
                    needs.push(VersionNeed {
                        file: file.clone(),
                        version: version.clone(),
                        weak,
                    });
                    version_names.insert(index, (version, file.clone(), weak));
                    let next = r.u32(aux + 12)?;
                    if next == 0 {
                        break;
                    }
                    aux += next as u64;
                }
                let next = r.u32(off + 12)?;
                if next == 0 {
                    break;
                }
                off += next as u64;
            }
        }
        if let Some(di) = dynsym_index {
            let ds = &sections[di];
            let strtab = sections.get(ds.link as usize).ok_or("bad dynsym link")?;
            let versym = sections.iter().find(|s| s.kind == SHT_GNU_VERSYM);
            if ds.entsize != 24 {
                return Err("unexpected dynamic symbol size".into());
            }
            let count = ds.size / 24;
            // Symbol index -> GOT slots, from the dynamic relocations.
            let mut slots: std::collections::BTreeMap<u64, Vec<u64>> = Default::default();
            for rel in sections
                .iter()
                .filter(|s| s.kind == SHT_RELA && s.link as usize == di)
            {
                if rel.entsize != 24 {
                    return Err("unexpected relocation size".into());
                }
                for k in 0..rel.size / 24 {
                    let o = rel.offset + k * 24;
                    let info = r.u64(o + 8)?;
                    let kind = (info & 0xffff_ffff) as u32;
                    if kind == R_X86_64_GLOB_DAT || kind == R_X86_64_JUMP_SLOT {
                        slots.entry(info >> 32).or_default().push(r.u64(o)?);
                    }
                }
            }
            for i in 1..count {
                let o = ds.offset + i * 24;
                if r.u16(o + 6)? != 0 {
                    continue; // defined here, not imported
                }
                let name = r.cstr(strtab, r.u32(o)? as u64)?;
                if name.is_empty() {
                    continue;
                }
                let bind = r.u8(o + 4)? >> 4;
                let ver = match versym {
                    Some(v) => version_names.get(&(r.u16(v.offset + i * 2)? & 0x7fff)),
                    None => None,
                };
                imports.push(Import {
                    name,
                    version: ver.map(|v| v.0.clone()),
                    file: ver.map(|v| v.1.clone()),
                    weak: bind == STB_WEAK || ver.is_some_and(|v| v.2),
                    slots: slots.remove(&i).unwrap_or_default(),
                });
            }
        }

        // Executable sections.
        let mut code = Vec::new();
        for s in &sections {
            if s.flags & SHF_EXECINSTR != 0 && s.kind != 8 && s.size > 0 {
                code.push(Code {
                    name: name(s)?,
                    addr: s.addr,
                    bytes: r.slice(s.offset, s.size)?,
                });
            }
        }

        // Notes and dynamic entries: prefer program headers (what the loader
        // reads), fall back to sections.
        let mut note_ranges = Vec::new();
        let mut dynamic = None;
        for i in 0..phnum {
            let o = phoff + i * 56;
            let (kind, off, size) = (r.u32(o)?, r.u64(o + 8)?, r.u64(o + 32)?);
            match kind {
                PT_NOTE => note_ranges.push((off, size)),
                PT_DYNAMIC => dynamic = Some((off, size)),
                _ => {}
            }
        }
        if note_ranges.is_empty() {
            note_ranges.extend(
                sections
                    .iter()
                    .filter(|s| s.kind == SHT_NOTE)
                    .map(|s| (s.offset, s.size)),
            );
        }
        let mut abi_tag = None;
        for (off, size) in note_ranges {
            let mut p = off;
            while p + 12 <= off + size {
                let (namesz, descsz, kind) =
                    (r.u32(p)? as u64, r.u32(p + 4)? as u64, r.u32(p + 8)?);
                let name_at = p + 12;
                let desc_at = name_at + namesz.div_ceil(4) * 4;
                if kind == NT_GNU_ABI_TAG
                    && namesz == 4
                    && r.slice(name_at, 4)? == b"GNU\0"
                    && descsz >= 16
                    && r.u32(desc_at)? == 0
                {
                    abi_tag = Some([
                        r.u32(desc_at + 4)?,
                        r.u32(desc_at + 8)?,
                        r.u32(desc_at + 12)?,
                    ]);
                }
                p = desc_at + descsz.div_ceil(4) * 4;
            }
        }
        let mut relr = false;
        if let Some((off, size)) = dynamic {
            for k in 0..size / 16 {
                let tag = r.u64(off + k * 16)? as i64;
                if tag == 0 {
                    break;
                }
                relr |= tag == DT_RELR;
            }
        }
        Ok(Elf {
            imports,
            needs,
            code,
            abi_tag,
            relr,
        })
    }
}
