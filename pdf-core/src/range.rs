use crate::error::{PdfError, Result};

/// A parsed, 1-based, inclusive page selection such as `1-3,5,8-10`.
///
/// Pages are stored normalized: sorted ascending and de-duplicated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRange {
    pages: Vec<usize>,
}

impl PageRange {
    /// Parse a human range spec like `"1-3,5,8-10"` against a document with
    /// `total` pages. Page numbers are 1-based and inclusive.
    pub fn parse(spec: &str, total: usize) -> Result<Self> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(PdfError::Invalid("empty page range".into()));
        }

        let mut pages = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match part.split_once('-') {
                Some((a, b)) => {
                    let start: usize = parse_num(a)?;
                    let end: usize = parse_num(b)?;
                    if start == 0 || end == 0 {
                        return Err(PdfError::Invalid("page numbers are 1-based".into()));
                    }
                    let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
                    if hi > total {
                        return Err(PdfError::PageOutOfRange(part.to_string(), total));
                    }
                    pages.extend(lo..=hi);
                }
                None => {
                    let n: usize = parse_num(part)?;
                    if n == 0 {
                        return Err(PdfError::Invalid("page numbers are 1-based".into()));
                    }
                    if n > total {
                        return Err(PdfError::PageOutOfRange(part.to_string(), total));
                    }
                    pages.push(n);
                }
            }
        }

        if pages.is_empty() {
            return Err(PdfError::Invalid("page range selected no pages".into()));
        }

        pages.sort_unstable();
        pages.dedup();
        Ok(PageRange { pages })
    }

    /// The selected 1-based page numbers, ascending and de-duplicated.
    pub fn pages(&self) -> &[usize] {
        &self.pages
    }

    /// Pages that are *not* selected, within `1..=total`.
    pub fn complement(&self, total: usize) -> Vec<usize> {
        let selected: std::collections::HashSet<usize> = self.pages.iter().copied().collect();
        (1..=total).filter(|p| !selected.contains(p)).collect()
    }
}

fn parse_num(s: &str) -> Result<usize> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| PdfError::Invalid(format!("invalid page number: {s:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mixed_spec() {
        let r = PageRange::parse("1-3,5,8-10", 12).unwrap();
        assert_eq!(r.pages(), &[1, 2, 3, 5, 8, 9, 10]);
    }

    #[test]
    fn normalizes_overlap_and_reverse() {
        let r = PageRange::parse("5-3,4,3", 10).unwrap();
        assert_eq!(r.pages(), &[3, 4, 5]);
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(PageRange::parse("1-99", 10).is_err());
    }

    #[test]
    fn rejects_zero() {
        assert!(PageRange::parse("0", 10).is_err());
    }

    #[test]
    fn complement_works() {
        let r = PageRange::parse("2,4", 5).unwrap();
        assert_eq!(r.complement(5), vec![1, 3, 5]);
    }
}
