//! A minimal PDF 1.4 writer: numbered objects (reserved first so pages can point forward), a
//! cross-reference table and a trailer. Enough for the film and job-sheet files; no compression.

use crate::error::{Result, param};
use std::fmt::Write as _;

pub(crate) struct Pdf {
    out: Vec<u8>,
    offsets: Vec<Option<usize>>,
}

impl Pdf {
    pub(crate) fn new() -> Self {
        Self { out: b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec(), offsets: Vec::new() }
    }

    /// The number of the next object, to be filled in with [`Pdf::set`].
    pub(crate) fn reserve(&mut self) -> usize {
        self.offsets.push(None);
        self.offsets.len()
    }

    pub(crate) fn set(&mut self, id: usize, body: &[u8]) {
        if let Some(slot) = id.checked_sub(1).and_then(|i| self.offsets.get_mut(i)) {
            *slot = Some(self.out.len());
            self.out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
            self.out.extend_from_slice(body);
            self.out.extend_from_slice(b"\nendobj\n");
        }
    }

    pub(crate) fn add(&mut self, body: &[u8]) -> usize {
        let id = self.reserve();
        self.set(id, body);
        id
    }

    /// Writes the cross-reference table; every reserved object must have been set.
    pub(crate) fn finish(mut self, root: usize) -> Result<Vec<u8>> {
        let xref = self.out.len();
        let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", self.offsets.len() + 1);
        for o in &self.offsets {
            let o = o.ok_or_else(|| param("pdf", "an object was reserved but never written"))?;
            let _ = writeln!(table, "{o:010} 00000 n ");
        }
        let _ = write!(table, "trailer\n<< /Size {} /Root {root} 0 R >>\nstartxref\n{xref}\n%%EOF\n", self.offsets.len() + 1);
        self.out.extend_from_slice(table.as_bytes());
        Ok(self.out)
    }
}

/// The body of a stream object: its dictionary entries (without `/Length`) and bytes.
pub(crate) fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unwritten_object_is_an_error_not_a_broken_file() {
        let mut p = Pdf::new();
        let root = p.add(b"<< /Type /Catalog >>");
        p.reserve();
        assert!(p.finish(root).is_err());
    }

    #[test]
    fn xref_offsets_point_at_their_objects() {
        let mut p = Pdf::new();
        let later = p.reserve();
        let root = p.add(b"<< /Type /Catalog >>");
        p.set(later, b"<< /Type /Pages >>");
        let bytes = p.finish(root).unwrap();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.starts_with("%PDF-1.4\n") && text.ends_with("%%EOF\n"));
        assert!(text.contains("/Size 3 /Root 2 0 R"));
        let xref_at = bytes.windows(9).position(|w| w == b"xref\n0 3\n").unwrap();
        let tail = String::from_utf8_lossy(&bytes[xref_at..]).into_owned();
        let rows: Vec<&str> = tail.lines().skip(2).take(3).collect();
        assert_eq!(rows[0], "0000000000 65535 f ");
        for (i, row) in rows.iter().skip(1).enumerate() {
            let off: usize = row[..10].parse().unwrap();
            assert!(bytes[off..].starts_with(format!("{} 0 obj\n", i + 1).as_bytes()), "object {}", i + 1);
        }
        let start: usize = tail.split("startxref\n").nth(1).unwrap().lines().next().unwrap().parse().unwrap();
        assert_eq!(start, xref_at);
    }

    #[test]
    fn setting_an_unreserved_id_is_ignored() {
        let mut p = Pdf::new();
        let root = p.add(b"<< >>");
        let before = p.out.clone();
        p.set(0, b"x");
        p.set(99, b"x");
        assert_eq!(p.out, before);
        assert!(p.finish(root).is_ok());
    }

    #[test]
    fn a_stream_states_its_exact_length() {
        let s = stream("/Filter /None", &[0, 1, 2, 255]);
        let text = String::from_utf8_lossy(&s).into_owned();
        assert!(text.starts_with("<< /Filter /None /Length 4 >>\nstream\n"));
        assert!(s.ends_with(b"\nendstream"));
        let empty = stream("", &[]);
        assert!(String::from_utf8_lossy(&empty).contains("/Length 0"));
    }
}
