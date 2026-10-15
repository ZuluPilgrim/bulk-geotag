//! Work around photos whose EXIF stores a standard tag with the wrong number type,
//! e.g. XResolution as a signed integer instead of a fraction. Most readers accept
//! these, but little_exif refuses to load the whole block ("Illegal format for known tag").
//!
//! Fix: in the temp copy only, give each badly typed entry a private tag number so
//! little_exif loads it as an unknown tag; after loading, drop those placeholders and
//! put back the same values as correctly typed tags. Nothing else in the EXIF changes.

use little_exif::endian::Endian;
use little_exif::exif_tag::ExifTag;
use little_exif::exif_tag_format::ExifTagFormat;
use little_exif::ifd::ExifTagGroup;
use little_exif::metadata::Metadata;

/// Private tag numbers used while loading; no standard tag lives in this range.
const PLACEHOLDER_BASE: u16 = 0xFE00;

/// One badly typed entry: where it is, its placeholder number, and the corrected tag.
#[derive(Debug)]
pub struct Repair {
    group: ExifTagGroup,
    ifd_nr: u32,
    placeholder: u16,
    fixed: ExifTag,
}

impl Repair {
    /// e.g. "XResolution" (used by the tests)
    #[cfg(test)]
    pub fn tag_name(&self) -> String {
        let s = format!("{:?}", self.fixed);
        s.split('(').next().unwrap_or(&s).to_string()
    }
}

/// True for the error little_exif gives when a known tag has the wrong type.
pub fn is_bad_type_error(msg: &str) -> bool {
    msg.contains("Illegal format for known tag")
}

/// Find the EXIF (TIFF) block `tiff` inside `file` and retag every badly typed entry
/// that can be converted safely. Returns what was changed; empty means nothing to fix.
pub fn neutralise(file: &mut [u8], tiff: &[u8]) -> Vec<Repair> {
    let Some(start) = find(file, tiff) else { return Vec::new() };
    let t = &mut file[start..start + tiff.len()];
    let little = match t.get(0..4) {
        Some(b"II*\0") => true,
        Some(b"MM\0*") => false,
        _ => return Vec::new(),
    };
    let mut repairs = Vec::new();
    let mut seen = Vec::new(); // offsets already walked, guards against loops
    let mut next = rd32(t, 4, little).map(|o| o as usize);
    let mut ifd_nr = 0u32;
    while let Some(off) = next.filter(|&o| o != 0) {
        if seen.contains(&off) || ifd_nr > 8 {
            break;
        }
        seen.push(off);
        next = walk(t, off, little, ExifTagGroup::GENERIC, ifd_nr, &mut repairs, &mut seen);
        ifd_nr += 1;
    }
    repairs
}

/// After loading: remove the placeholders and set the corrected tags in the same IFDs.
pub fn restore(md: &mut Metadata, repairs: Vec<Repair>) {
    for r in repairs {
        let ifd = md.get_ifd_mut(r.group, r.ifd_nr);
        ifd.remove_tag(r.placeholder);
        ifd.set_tag(r.fixed);
    }
}

/// Walk one IFD; returns the offset of the next IFD in the chain (generic IFDs only).
fn walk(
    t: &mut [u8],
    off: usize,
    little: bool,
    group: ExifTagGroup,
    ifd_nr: u32,
    repairs: &mut Vec<Repair>,
    seen: &mut Vec<usize>,
) -> Option<usize> {
    let count = rd16(t, off, little)? as usize;
    for i in 0..count {
        let e = off + 2 + i * 12;
        let (Some(tag), Some(typ), Some(n)) = (rd16(t, e, little), rd16(t, e + 2, little), rd32(t, e + 4, little)) else {
            return None;
        };
        // Sub-IFDs: EXIF and GPS hang off IFD0, Interop off the EXIF IFD.
        let sub = match (group, tag) {
            (ExifTagGroup::GENERIC, 0x8769) => Some(ExifTagGroup::EXIF),
            (ExifTagGroup::GENERIC, 0x8825) => Some(ExifTagGroup::GPS),
            (ExifTagGroup::EXIF, 0xA005) => Some(ExifTagGroup::INTEROP),
            _ => None,
        };
        if let Some(g) = sub {
            if let Some(o) = rd32(t, e + 8, little).map(|o| o as usize) {
                if o != 0 && !seen.contains(&o) {
                    seen.push(o);
                    walk(t, o, little, g, ifd_nr, repairs, seen);
                }
            }
            continue;
        }
        // Offset/size tags are rebuilt by little_exif itself; leave them alone.
        if matches!(tag, 0x0111 | 0x0117 | 0x0201 | 0x0202) {
            continue;
        }
        let Ok(known) = ExifTag::from_u16(tag, &group) else { continue };
        if known.is_unknown() {
            continue;
        }
        let want = known.format();
        if want.as_u16() == typ || tolerated(want.as_u16(), typ) {
            continue;
        }
        let Some(values) = read_ints(t, e, typ, n, little) else { continue };
        let Some(fixed) = convert(tag, &want, &values, &group) else { continue };
        let placeholder = PLACEHOLDER_BASE + repairs.len() as u16;
        wr16(t, e, placeholder, little);
        repairs.push(Repair { group, ifd_nr, placeholder, fixed });
    }
    rd32(t, off + 2 + count * 12, little).map(|o| o as usize)
}

/// Type mismatches little_exif already converts by itself (see its decode.rs).
fn tolerated(want: u16, got: u16) -> bool {
    matches!((want, got), (4, 3) | (4, 1) | (3, 1) | (3, 4) | (1, 3) | (1, 2) | (10, 5) | (7, 2))
}

/// The entry's values, if it is a whole-number type (byte/short/long, signed or not).
fn read_ints(t: &[u8], e: usize, typ: u16, n: u32, little: bool) -> Option<Vec<i64>> {
    let size = match typ {
        1 | 6 => 1,
        3 | 8 => 2,
        4 | 9 => 4,
        _ => return None,
    };
    let n = n as usize;
    if n == 0 || n > 64 {
        return None;
    }
    let total = size * n;
    let base = if total <= 4 { e + 8 } else { rd32(t, e + 8, little)? as usize };
    if base + total > t.len() {
        return None;
    }
    (0..n)
        .map(|i| {
            let p = base + i * size;
            Some(match typ {
                1 => t[p] as i64,
                6 => t[p] as i8 as i64,
                3 => rd16(t, p, little)? as i64,
                8 => rd16(t, p, little)? as i16 as i64,
                4 => rd32(t, p, little)? as i64,
                _ => rd32(t, p, little)? as i32 as i64,
            })
        })
        .collect()
}

/// Build the correctly typed tag from whole-number values, if the values fit.
fn convert(tag: u16, want: &ExifTagFormat, v: &[i64], group: &ExifTagGroup) -> Option<ExifTag> {
    let mut b = Vec::new();
    for &x in v {
        match want {
            ExifTagFormat::RATIONAL64U => {
                b.extend(u32::try_from(x).ok()?.to_le_bytes());
                b.extend(1u32.to_le_bytes());
            }
            ExifTagFormat::RATIONAL64S => {
                b.extend(i32::try_from(x).ok()?.to_le_bytes());
                b.extend(1i32.to_le_bytes());
            }
            ExifTagFormat::INT8U => b.push(u8::try_from(x).ok()?),
            ExifTagFormat::INT16U => b.extend(u16::try_from(x).ok()?.to_le_bytes()),
            ExifTagFormat::INT32U => b.extend(u32::try_from(x).ok()?.to_le_bytes()),
            ExifTagFormat::INT16S => b.extend(i16::try_from(x).ok()?.to_le_bytes()),
            ExifTagFormat::INT32S => b.extend(i32::try_from(x).ok()?.to_le_bytes()),
            _ => return None,
        }
    }
    ExifTag::from_u16_with_data(tag, want, &b, &Endian::Little, group).ok()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() < 8 || needle.len() > hay.len() {
        return None;
    }
    let head = &needle[..8];
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + 8] == head && &hay[i..i + needle.len()] == needle)
}

fn rd16(t: &[u8], p: usize, little: bool) -> Option<u16> {
    let b: [u8; 2] = t.get(p..p + 2)?.try_into().ok()?;
    Some(if little { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) })
}

fn rd32(t: &[u8], p: usize, little: bool) -> Option<u32> {
    let b: [u8; 4] = t.get(p..p + 4)?.try_into().ok()?;
    Some(if little { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
}

fn wr16(t: &mut [u8], p: usize, v: u16, little: bool) {
    let b = if little { v.to_le_bytes() } else { v.to_be_bytes() };
    t[p..p + 2].copy_from_slice(&b);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-entry big-endian TIFF block with XResolution stored as a signed long (72).
    fn bad_tiff() -> Vec<u8> {
        let mut t = b"MM\0*\0\0\0\x08".to_vec();
        t.extend([0, 1]); // one entry
        t.extend([0x01, 0x1A, 0, 9, 0, 0, 0, 1, 0, 0, 0, 72]); // XResolution, SLONG, 1, 72
        t.extend([0, 0, 0, 0]); // no next IFD
        t
    }

    #[test]
    fn badly_typed_xresolution_is_retagged() {
        let tiff = bad_tiff();
        let mut file = b"junk before".to_vec();
        file.extend(&tiff);
        let repairs = neutralise(&mut file, &tiff);
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].tag_name(), "XResolution");
        assert_eq!(&file[11 + 10..11 + 12], &PLACEHOLDER_BASE.to_be_bytes());
    }

    #[test]
    fn well_formed_exif_is_left_alone() {
        let mut tiff = bad_tiff();
        // Orientation stored as LONG instead of SHORT: little_exif converts this itself.
        tiff[11] = 0x12; // tag 0x0112
        tiff[13] = 4; // type LONG
        let mut file = tiff.clone();
        assert!(neutralise(&mut file, &tiff).is_empty());
        assert_eq!(file, tiff);
    }

    #[test]
    fn only_the_error_we_handle_is_recognised() {
        assert!(is_bad_type_error("Illegal format for known tag! Tag: XResolution([]) Expected: RATIONAL64U Got: INT32S"));
        assert!(!is_bad_type_error("No EXIF data found"));
    }
}
