use crate::{Position, SourceSpan};

#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<usize>,
    text_len: usize,
    physical_lines: u32,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' && index + 1 < text.len() {
                starts.push(index + 1);
            }
        }
        let physical_lines = if text.is_empty() {
            0
        } else {
            starts.len().try_into().unwrap_or(u32::MAX)
        };
        Self {
            starts,
            text_len: text.len(),
            physical_lines,
        }
    }

    pub fn physical_lines(&self) -> u32 {
        self.physical_lines
    }

    pub fn span(&self, text: &str, start: usize, end: usize) -> SourceSpan {
        let safe_start = start.min(self.text_len);
        let safe_end = end.min(self.text_len).max(safe_start);
        SourceSpan {
            byte_start: to_u32(safe_start),
            byte_end: to_u32(safe_end),
            start: self.position(text, safe_start),
            end: self.position(text, safe_end),
        }
    }

    pub fn span_line_count(&self, start: usize, end: usize) -> u32 {
        if end <= start {
            return 0;
        }
        let first = self.line_number(start.min(self.text_len));
        let last = self.line_number(end.saturating_sub(1).min(self.text_len));
        last.saturating_sub(first) + 1
    }

    pub fn line_text<'a>(&self, text: &'a str, line: u32) -> Option<&'a str> {
        let index = usize::try_from(line.checked_sub(1)?).ok()?;
        let start = *self.starts.get(index)?;
        let end = self.starts.get(index + 1).copied().unwrap_or(text.len());
        Some(text[start..end].trim_end_matches(['\r', '\n']))
    }

    fn position(&self, text: &str, offset: usize) -> Position {
        let line = self.line_number(offset);
        let start = self.starts[(line - 1) as usize];
        let column = text[start..offset].chars().count() + 1;
        Position {
            line,
            column: column.try_into().unwrap_or(u32::MAX),
        }
    }

    fn line_number(&self, offset: usize) -> u32 {
        let index = self.starts.partition_point(|start| *start <= offset);
        index.try_into().unwrap_or(u32::MAX).max(1)
    }
}

fn to_u32(value: usize) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}

#[cfg(test)]
#[path = "tests/lines.rs"]
mod tests;
