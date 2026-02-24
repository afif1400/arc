/// Arc parser — transforms token stream into AST.
/// Designed for maximum forgiveness: recovers from errors, skips bad lines,
/// always produces a (possibly partial) Document.
use crate::ast::*;
use crate::lexer::{Token, TokenKind};

pub struct ParseResult {
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn skip_newlines(&mut self) {
        while let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Newline || matches!(tok.kind, TokenKind::Comment(_)) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn skip_to_newline(&mut self) {
        while let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Newline {
                break;
            }
            self.pos += 1;
        }
    }

    fn expect_newline_or_end(&mut self) {
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Newline {
                self.pos += 1;
            } else if !self.at_end() && tok.kind != TokenKind::RBrace {
                // Not a newline and not end — warn and skip
                self.diagnostics.push(Diagnostic {
                    line: tok.line,
                    col: tok.col,
                    code: "W001".into(),
                    message: "Expected newline after statement".into(),
                    suggestion: None,
                    severity: Severity::Warning,
                });
                self.skip_to_newline();
            }
        }
    }

    fn current_span(&self) -> Span {
        if let Some(tok) = self.peek() {
            Span {
                line: tok.line,
                col: tok.col,
                len: tok.len,
            }
        } else if let Some(last) = self.tokens.last() {
            Span {
                line: last.line,
                col: last.col + last.len,
                len: 0,
            }
        } else {
            Span::default()
        }
    }

    fn warn(&mut self, line: usize, col: usize, code: &str, msg: &str) {
        self.diagnostics.push(Diagnostic {
            line,
            col,
            code: code.into(),
            message: msg.into(),
            suggestion: None,
            severity: Severity::Warning,
        });
    }

    fn error(
        &mut self,
        line: usize,
        col: usize,
        code: &str,
        msg: &str,
        suggestion: Option<String>,
    ) {
        self.diagnostics.push(Diagnostic {
            line,
            col,
            code: code.into(),
            message: msg.into(),
            suggestion,
            severity: Severity::Error,
        });
    }

    // ── Parsing ───────────────────────────────────────────────

    fn parse_document(&mut self) -> Document {
        let mut doc = Document::default();
        self.skip_newlines();

        while !self.at_end() {
            self.skip_newlines();
            if self.at_end() {
                break;
            }

            let tok = self.peek().unwrap();
            match &tok.kind {
                // Directive: @direction, @theme, @spacing
                TokenKind::At => {
                    if let Some(dir) = self.parse_directive() {
                        doc.directives.push(dir);
                    }
                }
                // Group
                TokenKind::Group => {
                    if let Some(group) = self.parse_group() {
                        doc.groups.push(group);
                    }
                }
                // Include
                TokenKind::Include => {
                    if let Some(inc) = self.parse_include() {
                        doc.includes.push(inc);
                    }
                }
                // Node type keyword → could be a node declaration OR start of a connection
                k if k.is_node_type() => {
                    self.parse_node_or_connection(&mut doc);
                }
                // Identifier → could be a connection (from an already-declared node)
                TokenKind::Ident(_) => {
                    self.parse_ident_line(&mut doc);
                }
                // Comment — already skipped in skip_newlines, but handle explicitly
                TokenKind::Comment(_) => {
                    self.pos += 1;
                }
                // Unknown or unexpected
                _ => {
                    let t = self.peek().unwrap();
                    self.error(t.line, t.col, "E001", "Unexpected token", None);
                    self.skip_to_newline();
                }
            }
        }

        doc
    }

    fn parse_directive(&mut self) -> Option<Directive> {
        let at_tok = self.advance().unwrap(); // consume @
        let span_line = at_tok.line;
        let span_col = at_tok.col;

        let name_tok = self.advance()?;
        let name_line = name_tok.line;
        let name_col = name_tok.col;
        let name = match &name_tok.kind {
            TokenKind::Ident(s) => s.to_lowercase(),
            _ => {
                self.error(
                    name_line,
                    name_col,
                    "E002",
                    "Expected directive name after @",
                    None,
                );
                self.skip_to_newline();
                return None;
            }
        };

        let value_tok = self.advance();
        let value = match value_tok.map(|t| &t.kind) {
            Some(TokenKind::Ident(s)) => s.clone(),
            Some(TokenKind::QuotedString(s)) => s.clone(),
            _ => {
                self.error(
                    span_line,
                    span_col,
                    "E003",
                    &format!("Expected value for @{}", name),
                    None,
                );
                self.skip_to_newline();
                return None;
            }
        };

        self.expect_newline_or_end();

        match name.as_str() {
            "direction" | "dir" => match value.to_lowercase().as_str() {
                "down" | "vertical" | "tb" | "top-bottom" => {
                    Some(Directive::Direction(Direction::Down))
                }
                "right" | "horizontal" | "lr" | "left-right" => {
                    Some(Directive::Direction(Direction::Right))
                }
                _ => {
                    self.warn(
                        span_line,
                        span_col,
                        "W002",
                        &format!("Unknown direction '{}', using 'down'", value),
                    );
                    Some(Directive::Direction(Direction::Down))
                }
            },
            "theme" => Some(Directive::Theme(value.to_lowercase())),
            "spacing" => match value.to_lowercase().as_str() {
                "compact" => Some(Directive::Spacing(Spacing::Compact)),
                "normal" => Some(Directive::Spacing(Spacing::Normal)),
                "wide" => Some(Directive::Spacing(Spacing::Wide)),
                _ => {
                    self.warn(
                        span_line,
                        span_col,
                        "W003",
                        &format!("Unknown spacing '{}', using 'normal'", value),
                    );
                    Some(Directive::Spacing(Spacing::Normal))
                }
            },
            _ => {
                self.warn(
                    span_line,
                    span_col,
                    "W004",
                    &format!("Unknown directive '@{}'", name),
                );
                None
            }
        }
    }

    fn parse_group(&mut self) -> Option<Group> {
        let group_tok = self.advance().unwrap(); // consume 'group'
        let span = Span {
            line: group_tok.line,
            col: group_tok.col,
            len: group_tok.len,
        };

        // Label (required)
        let label = match self.peek().map(|t| &t.kind) {
            Some(TokenKind::QuotedString(s)) => {
                let s = s.clone();
                self.advance();
                s
            }
            Some(TokenKind::Ident(s)) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => {
                self.error(span.line, span.col, "E004", "Expected group label", None);
                self.skip_to_newline();
                return None;
            }
        };

        // Optional tags
        let tags = self.try_parse_tags();

        // Opening brace
        self.skip_newlines();
        match self.peek().map(|t| &t.kind) {
            Some(TokenKind::LBrace) => {
                self.advance();
            }
            _ => {
                self.error(
                    span.line,
                    span.col,
                    "E005",
                    "Expected '{' after group label",
                    None,
                );
                self.skip_to_newline();
                return None;
            }
        }

        // Members
        let mut members = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_end() {
                break;
            }

            match self.peek().map(|t| &t.kind) {
                Some(TokenKind::RBrace) => {
                    self.advance();
                    break;
                }
                Some(TokenKind::Group) => {
                    if let Some(sub) = self.parse_group() {
                        members.push(GroupMember::Group(sub));
                    }
                }
                Some(k) if k.is_node_type() => {
                    // Could be node declaration or a connection starting with type
                    let saved = self.pos;
                    if let Some(node) = self.try_parse_node_decl() {
                        // Check if next non-ws token is an arrow → it's actually a connection
                        if let Some(tok) = self.peek() {
                            if tok.kind.is_arrow() {
                                // Rewind and parse as connection
                                self.pos = saved;
                                let mut temp_doc = Document::default();
                                self.parse_node_or_connection(&mut temp_doc);
                                for n in temp_doc.nodes {
                                    members.push(GroupMember::Node(n));
                                }
                                for c in temp_doc.connections {
                                    members.push(GroupMember::Connection(c));
                                }
                                continue;
                            }
                        }
                        members.push(GroupMember::Node(node));
                    }
                }
                Some(TokenKind::Ident(_)) => {
                    // Could be node ref, comma-separated list, or connection
                    self.parse_group_ident_line(&mut members);
                }
                Some(TokenKind::Comment(_)) => {
                    self.advance();
                }
                _ => {
                    if let Some(t) = self.peek() {
                        self.error(t.line, t.col, "E006", "Unexpected token in group", None);
                    }
                    self.skip_to_newline();
                }
            }
        }

        Some(Group {
            label,
            tags,
            members,
            span,
        })
    }

    fn parse_group_ident_line(&mut self, members: &mut Vec<GroupMember>) {
        // Peek ahead: is this `Ident -> ...` (connection) or `Ident, Ident, ...` (ref list)?
        let first_ident = match &self.peek().unwrap().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return,
        };
        let _first_span = self.current_span();

        // Look ahead past ident
        let saved = self.pos;
        self.advance(); // consume ident

        match self.peek().map(|t| &t.kind) {
            Some(k) if k.is_arrow() => {
                // It's a connection
                self.pos = saved;
                let mut temp_doc = Document::default();
                self.parse_ident_line(&mut temp_doc);
                for c in temp_doc.connections {
                    members.push(GroupMember::Connection(c));
                }
            }
            Some(TokenKind::Comma) => {
                // It's a comma-separated list of refs
                let mut refs = vec![first_ident];
                while let Some(tok) = self.peek() {
                    if tok.kind == TokenKind::Comma {
                        self.advance();
                        if let Some(next) = self.peek() {
                            match &next.kind {
                                TokenKind::Ident(s) => {
                                    refs.push(s.clone());
                                    self.advance();
                                }
                                k if k.is_node_type() => {
                                    // node type used as ident ref in group
                                    refs.push(format!("{:?}", k));
                                    self.advance();
                                }
                                _ => break,
                            }
                        }
                    } else {
                        break;
                    }
                }
                if refs.len() == 1 {
                    members.push(GroupMember::NodeRef(refs.into_iter().next().unwrap()));
                } else {
                    members.push(GroupMember::NodeRefList(refs));
                }
                self.expect_newline_or_end();
            }
            _ => {
                // Single node ref
                members.push(GroupMember::NodeRef(first_ident));
                self.expect_newline_or_end();
            }
        }
    }

    fn parse_include(&mut self) -> Option<Include> {
        let inc_tok = self.advance().unwrap(); // consume 'include'
        let span = Span {
            line: inc_tok.line,
            col: inc_tok.col,
            len: inc_tok.len,
        };

        let path = match self.peek().map(|t| &t.kind) {
            Some(TokenKind::QuotedString(s)) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => {
                self.error(
                    span.line,
                    span.col,
                    "E007",
                    "Expected quoted path after 'include'",
                    None,
                );
                self.skip_to_newline();
                return None;
            }
        };

        self.expect_newline_or_end();
        Some(Include { path, span })
    }

    /// Parse a line that starts with a node type keyword.
    /// It could be:
    ///   - A node declaration: `service Auth "Auth Service" [tags]`
    ///   - A node declaration followed by arrow: `service Auth -> db Users`
    fn parse_node_or_connection(&mut self, doc: &mut Document) {
        // Parse the node first
        let node = match self.try_parse_node_decl() {
            Some(n) => n,
            None => {
                self.skip_to_newline();
                return;
            }
        };

        let node_id = node.id.clone();

        // Check if followed by an arrow → connection
        if let Some(tok) = self.peek() {
            if tok.kind.is_arrow() {
                // This node declaration is also the source of a connection
                doc.nodes.push(node);
                // Parse connection(s)
                while let Some(tok) = self.peek() {
                    if !tok.kind.is_arrow() {
                        break;
                    }
                    if let Some(conn) = self.parse_connection_from(&node_id) {
                        // If the target is a new node declaration, add it too
                        doc.connections.push(conn);
                    }
                }
                self.expect_newline_or_end();
                return;
            }
        }

        doc.nodes.push(node);
        self.expect_newline_or_end();
    }

    /// Try to parse a node declaration: TYPE IDENT [LABEL] [TAGS]
    fn try_parse_node_decl(&mut self) -> Option<Node> {
        let type_tok = self.peek()?;
        if !type_tok.kind.is_node_type() {
            return None;
        }

        let span = Span {
            line: type_tok.line,
            col: type_tok.col,
            len: type_tok.len,
        };
        let node_type = type_tok.kind.to_node_type().unwrap();
        self.advance(); // consume type

        // ID (required)
        let id = match self.peek().map(|t| &t.kind) {
            Some(TokenKind::Ident(s)) => {
                let s = s.clone();
                self.advance();
                s
            }
            Some(TokenKind::QuotedString(s)) => {
                // Forgiving: if someone puts a quoted string as ID, use it
                let s = s.clone();
                self.advance();
                // Generate a sanitized ID from the string
                s.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                    .collect::<String>()
            }
            _ => {
                self.error(
                    span.line,
                    span.col,
                    "E008",
                    &format!("Expected name after '{}'", node_type.as_str()),
                    None,
                );
                return None;
            }
        };

        // Optional label
        let label = match self.peek().map(|t| &t.kind) {
            Some(TokenKind::QuotedString(s)) => {
                let s = s.clone();
                self.advance();
                Some(s)
            }
            _ => None,
        };

        // Optional tags
        let tags = self.try_parse_tags();

        Some(Node {
            node_type,
            id,
            label,
            tags,
            span,
        })
    }

    /// Parse a connection starting from a known source ID.
    fn parse_connection_from(&mut self, from_id: &str) -> Option<Connection> {
        let arrow_tok = self.peek()?;
        if !arrow_tok.kind.is_arrow() {
            return None;
        }

        let span = Span {
            line: arrow_tok.line,
            col: arrow_tok.col,
            len: arrow_tok.len,
        };
        let arrow = arrow_tok.kind.to_arrow_kind().unwrap();
        self.advance(); // consume arrow

        // Target: could be `TYPE ID` or just `ID`
        let (to_id, _target_node) = self.parse_connection_target()?;

        // Optional label
        let label = self.try_parse_label();

        // Optional tags
        let tags = self.try_parse_tags();

        Some(Connection {
            from: from_id.to_string(),
            arrow,
            to: to_id,
            label,
            tags,
            span,
        })
    }

    /// Parse connection target: `TYPE ID` or just `ID`
    fn parse_connection_target(&mut self) -> Option<(String, Option<Node>)> {
        let is_node_type = self.peek().map(|t| t.kind.is_node_type()).unwrap_or(false);

        if is_node_type {
            // `TYPE ID` — inline node declaration as target
            if let Some(node) = self.try_parse_node_decl() {
                let id = node.id.clone();
                return Some((id, Some(node)));
            }
        }

        // Just an identifier
        let tok_line = self.peek().map(|t| t.line).unwrap_or(0);
        let tok_col = self.peek().map(|t| t.col).unwrap_or(0);
        let tok_kind = self.peek().map(|t| t.kind.clone());

        match tok_kind {
            Some(TokenKind::Ident(s)) => {
                self.advance();
                Some((s, None))
            }
            _ => {
                self.error(
                    tok_line,
                    tok_col,
                    "E009",
                    "Expected target node in connection",
                    None,
                );
                self.skip_to_newline();
                None
            }
        }
    }

    /// Parse a line starting with an identifier (likely a connection).
    fn parse_ident_line(&mut self, doc: &mut Document) {
        let ident_line = self.peek().map(|t| t.line).unwrap_or(0);
        let ident_col = self.peek().map(|t| t.col).unwrap_or(0);
        let id = match self.peek().map(|t| t.kind.clone()) {
            Some(TokenKind::Ident(s)) => s,
            _ => return,
        };
        self.advance(); // consume ident

        // Check for arrow
        if let Some(tok) = self.peek() {
            if tok.kind.is_arrow() {
                if let Some(conn) = self.parse_connection_from(&id) {
                    doc.connections.push(conn);
                }
                self.expect_newline_or_end();
                return;
            }
        }

        // Not a connection — it's an error (bare identifier on a line)
        if let Some(tok) = self.peek() {
            if tok.kind != TokenKind::Newline {
                self.warn(ident_line, ident_col, "W005",
                    &format!("'{}' appears as a bare identifier. Did you mean to declare a node? Use: service {}", id, id));
            }
        }
        self.skip_to_newline();
    }

    // ── Helpers ───────────────────────────────────────────────

    fn try_parse_tags(&mut self) -> Vec<String> {
        let mut tags = Vec::new();
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::LBracket {
                self.advance(); // consume [
                loop {
                    match self.peek().map(|t| &t.kind) {
                        Some(TokenKind::RBracket) => {
                            self.advance();
                            break;
                        }
                        Some(TokenKind::Comma) => {
                            self.advance();
                        }
                        Some(TokenKind::Ident(s)) => {
                            tags.push(s.clone());
                            self.advance();
                        }
                        Some(TokenKind::QuotedString(s)) => {
                            tags.push(s.clone());
                            self.advance();
                        }
                        // Accept node type keywords as tag values
                        Some(k) if k.is_node_type() => {
                            let type_name = match k {
                                TokenKind::Service => "service",
                                TokenKind::Db => "db",
                                TokenKind::Cache => "cache",
                                TokenKind::Queue => "queue",
                                TokenKind::Gateway => "gateway",
                                TokenKind::User => "user",
                                TokenKind::Store => "store",
                                TokenKind::Fn => "fn",
                                TokenKind::Worker => "worker",
                                TokenKind::External => "external",
                                _ => unreachable!(),
                            };
                            tags.push(type_name.to_string());
                            self.advance();
                        }
                        None => break,
                        _ => {
                            // Skip unknown tokens in tags
                            self.advance();
                        }
                    }
                }
            }
        }
        tags
    }

    fn try_parse_label(&mut self) -> Option<String> {
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Colon {
                self.advance(); // consume :
                match self.peek().map(|t| &t.kind) {
                    Some(TokenKind::QuotedString(s)) => {
                        let s = s.clone();
                        self.advance();
                        return Some(s);
                    }
                    // Forgiving: accept unquoted label (until next special token)
                    Some(TokenKind::Ident(s)) => {
                        let s = s.clone();
                        self.advance();
                        return Some(s);
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

// ── Public API ───────────────────────────────────────────────────

pub fn parse(input: &str) -> ParseResult {
    let tokens = crate::lexer::tokenize(input);
    let mut parser = Parser::new(tokens);
    let document = parser.parse_document();
    ParseResult {
        document,
        diagnostics: parser.diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_nodes() {
        let result = parse("service Auth\ndb Postgres\n");
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.document.nodes.len(), 2);
        assert_eq!(result.document.nodes[0].id, "Auth");
        assert_eq!(result.document.nodes[1].id, "Postgres");
    }

    #[test]
    fn test_connection() {
        let result = parse("Auth -> API: \"validate\"\n");
        assert_eq!(result.document.connections.len(), 1);
        assert_eq!(result.document.connections[0].from, "Auth");
        assert_eq!(result.document.connections[0].to, "API");
        assert_eq!(
            result.document.connections[0].label.as_deref(),
            Some("validate")
        );
    }

    #[test]
    fn test_node_with_label_and_tags() {
        let result = parse("service API \"API Gateway\" [Go, v3]\n");
        assert_eq!(result.document.nodes.len(), 1);
        let node = &result.document.nodes[0];
        assert_eq!(node.id, "API");
        assert_eq!(node.label.as_deref(), Some("API Gateway"));
        assert_eq!(node.tags, vec!["Go", "v3"]);
    }

    #[test]
    fn test_group() {
        let result = parse("group \"AWS\" {\n  Auth\n  API\n}\n");
        assert_eq!(result.document.groups.len(), 1);
        assert_eq!(result.document.groups[0].label, "AWS");
        assert_eq!(result.document.groups[0].members.len(), 2);
    }

    #[test]
    fn test_directive() {
        let result = parse("@direction right\n@theme dark\n");
        assert_eq!(result.document.direction(), Direction::Right);
        assert_eq!(result.document.theme_name(), "dark");
    }
}
