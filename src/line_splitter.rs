#[inline]
pub fn split_lines(src: &str) -> impl Iterator<Item=&str> {
    AgnostigLineSplitter::new(src)
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AgnostigLineSplitter<'a> {
    lines: &'a str
}

impl<'a> AgnostigLineSplitter<'a> {
    #[inline]
    pub fn new(lines: &'a str) -> Self {
        Self { lines }
    }
}

impl<'a> Iterator for AgnostigLineSplitter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.lines.char_indices();
        while let Some((index, ch)) = iter.next() {
            if ch == '\n' {
                let (head, tail) = self.lines.split_at(index);
                self.lines = &tail[1..];
                return Some(head);
            } else if ch == '\r' {
                if let Some((_, next_ch)) = iter.next() {
                    if next_ch == '\n' {
                        let (head, tail) = self.lines.split_at(index);
                        self.lines = &tail[2..];
                        return Some(head);
                    }
                }
                let (head, tail) = self.lines.split_at(index);
                self.lines = &tail[1..];
                return Some(head);
            }
        }
        None
    }
}
