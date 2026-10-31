pub mod activity;
pub mod class;
pub mod gantt;
pub mod mindmap;
pub mod ports;
pub mod sequence;
pub mod state;
pub mod sugiyama;
pub mod timing;
pub mod usecase;

/// The display lines of a PlantUML text, splitting where PlantUML breaks
/// a line: at a `\n` escape and at a real newline. `\\` is an escaped
/// backslash (so `\\n` shows as a backslash and an `n`, not a break), and
/// any other backslash is kept as written. An empty text has no lines.
pub fn text_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.peek() {
                Some('n') => {
                    chars.next();
                    lines.push(std::mem::take(&mut current));
                }
                Some('\\') => {
                    chars.next();
                    current.push('\\');
                }
                _ => current.push('\\'),
            },
            '\n' => lines.push(std::mem::take(&mut current)),
            '\r' => {}
            _ => current.push(c),
        }
    }
    lines.push(current);
    lines
}

#[cfg(test)]
mod tests {
    use super::text_lines;

    #[test]
    fn backslash_n_breaks_a_line() {
        assert_eq!(
            text_lines("New Order\\n http://sales/orders/1234"),
            vec!["New Order", " http://sales/orders/1234"]
        );
        assert_eq!(text_lines("a\\n\\nb"), vec!["a", "", "b"]);
        assert_eq!(text_lines("a\nb"), vec!["a", "b"]);
    }

    #[test]
    fn escaped_and_lone_backslashes_are_text() {
        assert_eq!(text_lines("C:\\\\new"), vec!["C:\\new"]);
        assert_eq!(text_lines("a\\tb\\"), vec!["a\\tb\\"]);
        assert_eq!(text_lines("plain"), vec!["plain"]);
        assert!(text_lines("").is_empty());
    }
}
