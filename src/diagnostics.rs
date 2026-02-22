use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: Option<Span>,
    pub source: Option<String>,
}

impl Diagnostic {
    pub fn new(code: &'static str, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code,
            message: message.into(),
            span,
            source: None,
        }
    }

    pub fn hint(&self) -> Option<&'static str> {
        hint_for_code(self.code)
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hint = self.hint();
        if let Some(source) = self.source() {
            write!(f, "{}: ", source)?;
        }
        if let Some(span) = &self.span {
            write!(
                f,
                "{}: {} at {}:{}",
                self.code, self.message, span.line, span.column
            )?;
        } else {
            write!(f, "{}: {}", self.code, self.message)?;
        }
        if let Some(hint) = hint {
            write!(f, " (hint: {hint})")?;
        }
        Ok(())
    }
}

pub fn hint_for_code(code: &str) -> Option<&'static str> {
    match code {
        "E-IO" => Some("入力ファイルのパスと読み取り権限を確認してください。"),
        "E-IMPORT" => Some("import パスと循環依存の有無を確認してください。"),
        "E-PARSE" => Some("S式の括弧対応とフォーム構造を確認してください。"),
        "E-RESOLVE" => Some("sort/relation/関数名の定義漏れや重複定義を確認してください。"),
        "E-STRATIFY" => Some("否定依存サイクルを除去し、層化可能な規則に分割してください。"),
        "E-TYPE" => Some("関数境界注釈と引数・戻り値の整合性を確認してください。"),
        "E-ENTAIL" => {
            Some("Refinement の前提事実・規則を追加し、含意が導出可能か確認してください。")
        }
        "E-TOTAL" => Some("再帰呼び出しを除去し、全関数（停止性保証）として定義してください。"),
        "E-DATA" => Some("data 宣言の重複・再帰・constructor の整合性を確認してください。"),
        "E-MATCH" => Some("match の網羅性・到達不能分岐・パターン型整合性を確認してください。"),
        "E-PROVE" => Some("universe と証明義務を確認し、反例トレースを参照して修正してください。"),
        _ => None,
    }
}

pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in src.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub fn make_span(src: &str, start: usize, end: usize) -> Span {
    make_span_with_file(src, start, end, None)
}

pub fn make_span_with_file(src: &str, start: usize, end: usize, file_id: Option<&str>) -> Span {
    let (line, column) = line_col(src, start);
    Span {
        start,
        end,
        line,
        column,
        file_id: file_id.map(str::to_string),
    }
}
