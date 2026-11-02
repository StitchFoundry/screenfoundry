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
        let a = p.add(b"<< /Type /Catalog >>");
        let b = p.add(b"<< /N 2 >>");
        let bytes = p.finish(a).unwrap();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        // The header carries non-UTF-8 bytes, so offsets are checked against `bytes`, not `text`.
        let xref = bytes.windows(6).rposition(|w| w == b"\nxref\n").unwrap() + 1;
        let tail = String::from_utf8_lossy(&bytes[xref..]).into_owned();
        let rows: Vec<&str> = tail.lines().skip(3).take(2).collect();
        for (row, id) in rows.iter().zip([a, b]) {
            let off: usize = row[..10].parse().unwrap();
            assert!(bytes[off..].starts_with(format!("{id} 0 obj\n").as_bytes()));
        }
        assert!(text.contains("/Size 3 /Root 1 0 R"));
        let start: usize = tail.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
        assert_eq!(start, xref);
        assert!(text.ends_with("%%EOF\n"));
    }

    #[test]
    fn set_with_an_unknown_id_is_ignored_and_forward_references_work() {
        let mut p = Pdf::new();
        let id = p.reserve();
        p.set(0, b"x");
        p.set(99, b"x");
        assert!(p.out.len() == Pdf::new().out.len(), "bad ids must write nothing");
        p.set(id, b"<< >>");
        assert!(p.finish(id).is_ok());
    }

    #[test]
    fn an_empty_document_still_has_a_valid_trailer() {
        let bytes = Pdf::new().finish(1).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("0 1\n0000000000 65535 f"));
    }

    #[test]
    fn stream_length_counts_binary_bytes() {
        let data = [0u8, 255, 10, 13, 0];
        let s = stream("/Filter /X", &data);
        let head = String::from_utf8_lossy(&s[..30]).into_owned();
        assert!(head.contains("/Length 5 >>"));
        assert!(s.ends_with(b"\nendstream"));
        assert!(stream("", &[]).starts_with(b"<<  /Length 0 >>"));
    }
}
