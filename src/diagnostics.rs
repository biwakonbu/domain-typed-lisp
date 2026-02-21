use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn new(code: &'static str, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = &self.span {
            write!(
                f,
                "{}: {} at {}:{}",
                self.code, self.message, span.line, span.column
            )
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}

pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, b) in src.as_bytes().iter().enumerate() {
        if i >= offset {
            break;
        }
        if *b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub fn make_span(src: &str, start: usize, end: usize) -> Span {
    let (line, column) = line_col(src, start);
    Span {
        start,
        end,
        line,
        column,
    }
}
