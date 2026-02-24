/// Arc lexer — tokenizes .arc source into a flat stream of tokens.
/// Designed for maximum error recovery: never panics, always produces tokens.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords / types
    Service, Db, Cache, Queue, Gateway, User, Store, Fn, Worker, External,
    Group, Include,
    // Directives
    At,          // @
    // Arrows
    Arrow,       // ->
    DashedArrow, // -->
    BiArrow,     // <->
    BlockArrow,  // -x
    // Delimiters
    LBrace,      // {
    RBrace,      // }
    LBracket,    // [
    RBracket,    // ]
    LParen,      // (
    RParen,      // )
    Colon,       // :
    Comma,       // ,
    // Literals
    QuotedString(String),
    Ident(String),
    // Structure
    Newline,
    Comment(String),
    // Error recovery
    Unknown(char),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while pos < chars.len() {
        let ch = chars[pos];

        // Skip spaces and tabs (not newlines)
        if ch == ' ' || ch == '\t' {
            pos += 1;
            col += 1;
            continue;
        }

        // Newlines
        if ch == '\n' {
            tokens.push(Token { kind: TokenKind::Newline, line, col, len: 1 });
            pos += 1;
            line += 1;
            col = 1;
            continue;
        }
        if ch == '\r' {
            pos += 1;
            if pos < chars.len() && chars[pos] == '\n' {
                pos += 1;
            }
            tokens.push(Token { kind: TokenKind::Newline, line, col, len: 1 });
            line += 1;
            col = 1;
            continue;
        }

        // Comments
        if ch == '#' {
            let start_col = col;
            let start = pos;
            pos += 1;
            col += 1;
            while pos < chars.len() && chars[pos] != '\n' && chars[pos] != '\r' {
                pos += 1;
                col += 1;
            }
            let text: String = chars[start + 1..pos].iter().collect();
            tokens.push(Token { kind: TokenKind::Comment(text.trim().to_string()), line, col: start_col, len: pos - start });
            continue;
        }

        // Quoted strings
        if ch == '"' {
            let start_col = col;
            let start = pos;
            pos += 1;
            col += 1;
            let mut s = String::new();
            while pos < chars.len() && chars[pos] != '"' && chars[pos] != '\n' {
                if chars[pos] == '\\' && pos + 1 < chars.len() {
                    match chars[pos + 1] {
                        'n' => { s.push('\n'); pos += 2; col += 2; continue; }
                        '"' => { s.push('"'); pos += 2; col += 2; continue; }
                        '\\' => { s.push('\\'); pos += 2; col += 2; continue; }
                        _ => {}
                    }
                }
                s.push(chars[pos]);
                pos += 1;
                col += 1;
            }
            if pos < chars.len() && chars[pos] == '"' {
                pos += 1;
                col += 1;
            }
            // Forgiving: if string not closed, still produce token
            tokens.push(Token { kind: TokenKind::QuotedString(s), line, col: start_col, len: pos - start });
            continue;
        }

        // Arrows: ->, -->, <->, -x
        if ch == '-' {
            let start_col = col;
            if pos + 1 < chars.len() && chars[pos + 1] == '>' {
                tokens.push(Token { kind: TokenKind::Arrow, line, col: start_col, len: 2 });
                pos += 2; col += 2;
                continue;
            }
            if pos + 2 < chars.len() && chars[pos + 1] == '-' && chars[pos + 2] == '>' {
                tokens.push(Token { kind: TokenKind::DashedArrow, line, col: start_col, len: 3 });
                pos += 3; col += 3;
                continue;
            }
            if pos + 1 < chars.len() && chars[pos + 1] == 'x' {
                tokens.push(Token { kind: TokenKind::BlockArrow, line, col: start_col, len: 2 });
                pos += 2; col += 2;
                continue;
            }
            // Just a dash — part of an ident? Fall through to ident
        }
        if ch == '<' && pos + 2 < chars.len() && chars[pos + 1] == '-' && chars[pos + 2] == '>' {
            let start_col = col;
            tokens.push(Token { kind: TokenKind::BiArrow, line, col: start_col, len: 3 });
            pos += 3; col += 3;
            continue;
        }

        // Single char tokens
        match ch {
            '@' => { tokens.push(Token { kind: TokenKind::At, line, col, len: 1 }); pos += 1; col += 1; continue; }
            '{' => { tokens.push(Token { kind: TokenKind::LBrace, line, col, len: 1 }); pos += 1; col += 1; continue; }
            '}' => { tokens.push(Token { kind: TokenKind::RBrace, line, col, len: 1 }); pos += 1; col += 1; continue; }
            '[' => { tokens.push(Token { kind: TokenKind::LBracket, line, col, len: 1 }); pos += 1; col += 1; continue; }
            ']' => { tokens.push(Token { kind: TokenKind::RBracket, line, col, len: 1 }); pos += 1; col += 1; continue; }
            '(' => { tokens.push(Token { kind: TokenKind::LParen, line, col, len: 1 }); pos += 1; col += 1; continue; }
            ')' => { tokens.push(Token { kind: TokenKind::RParen, line, col, len: 1 }); pos += 1; col += 1; continue; }
            ':' => { tokens.push(Token { kind: TokenKind::Colon, line, col, len: 1 }); pos += 1; col += 1; continue; }
            ',' => { tokens.push(Token { kind: TokenKind::Comma, line, col, len: 1 }); pos += 1; col += 1; continue; }
            _ => {}
        }

        // Identifiers and keywords
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            let start_col = col;
            let start = pos;
            while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_' || chars[pos] == '-') {
                pos += 1;
                col += 1;
            }
            let word: String = chars[start..pos].iter().collect();
            let len = pos - start;
            // Only match EXACT canonical type names as keywords.
            // Aliases (svc, database, client, etc.) are handled by
            // NodeType::from_str_fuzzy in the parser — NOT in the lexer.
            // This prevents words like "Client" from being misidentified.
            let kind = match word.as_str() {
                "service" => TokenKind::Service,
                "db" => TokenKind::Db,
                "cache" => TokenKind::Cache,
                "queue" => TokenKind::Queue,
                "gateway" => TokenKind::Gateway,
                "user" => TokenKind::User,
                "store" => TokenKind::Store,
                "fn" => TokenKind::Fn,
                "worker" => TokenKind::Worker,
                "external" => TokenKind::External,
                "group" => TokenKind::Group,
                "include" => TokenKind::Include,
                _ => TokenKind::Ident(word),
            };
            tokens.push(Token { kind, line, col: start_col, len });
            continue;
        }

        // Unknown character — error recovery: skip it
        tokens.push(Token { kind: TokenKind::Unknown(ch), line, col, len: 1 });
        pos += 1;
        col += 1;
    }

    tokens
}

impl TokenKind {
    pub fn is_node_type(&self) -> bool {
        matches!(self,
            TokenKind::Service | TokenKind::Db | TokenKind::Cache |
            TokenKind::Queue | TokenKind::Gateway | TokenKind::User |
            TokenKind::Store | TokenKind::Fn | TokenKind::Worker |
            TokenKind::External
        )
    }

    pub fn to_node_type(&self) -> Option<crate::ast::NodeType> {
        match self {
            TokenKind::Service => Some(crate::ast::NodeType::Service),
            TokenKind::Db => Some(crate::ast::NodeType::Db),
            TokenKind::Cache => Some(crate::ast::NodeType::Cache),
            TokenKind::Queue => Some(crate::ast::NodeType::Queue),
            TokenKind::Gateway => Some(crate::ast::NodeType::Gateway),
            TokenKind::User => Some(crate::ast::NodeType::User),
            TokenKind::Store => Some(crate::ast::NodeType::Store),
            TokenKind::Fn => Some(crate::ast::NodeType::Fn),
            TokenKind::Worker => Some(crate::ast::NodeType::Worker),
            TokenKind::External => Some(crate::ast::NodeType::External),
            _ => None,
        }
    }

    pub fn is_arrow(&self) -> bool {
        matches!(self,
            TokenKind::Arrow | TokenKind::DashedArrow |
            TokenKind::BiArrow | TokenKind::BlockArrow
        )
    }

    pub fn to_arrow_kind(&self) -> Option<crate::ast::ArrowKind> {
        match self {
            TokenKind::Arrow => Some(crate::ast::ArrowKind::Solid),
            TokenKind::DashedArrow => Some(crate::ast::ArrowKind::Dashed),
            TokenKind::BiArrow => Some(crate::ast::ArrowKind::Bidirectional),
            TokenKind::BlockArrow => Some(crate::ast::ArrowKind::Blocked),
            _ => None,
        }
    }
}
