/// Arc validator — semantic validation with machine-readable JSON output.
/// Designed for agent repair loops: clear error codes, actionable suggestions.

use crate::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<Diagnostic>,
    pub fixable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_source: Option<String>,
}

/// Validate a parsed document. Returns diagnostics and a resolved document
/// where undeclared nodes are added as implicit `service` nodes.
pub fn validate(doc: &Document, parse_diagnostics: &[Diagnostic]) -> (ValidationResult, Document) {
    let mut diagnostics: Vec<Diagnostic> = parse_diagnostics.to_vec();
    let mut resolved = doc.clone();

    // 1. Check for duplicate node IDs
    let mut seen_ids: HashMap<&str, &Node> = HashMap::new();
    for node in &doc.nodes {
        if let Some(existing) = seen_ids.get(node.id.as_str()) {
            diagnostics.push(Diagnostic {
                line: node.span.line,
                col: node.span.col,
                code: "E010".into(),
                message: format!("Duplicate node ID '{}' (first declared at line {})", node.id, existing.span.line),
                suggestion: Some(format!("Rename to '{}_2' or use a unique identifier", node.id)),
                severity: Severity::Error,
            });
        } else {
            seen_ids.insert(&node.id, node);
        }
    }

    // 2. Check for undeclared nodes in connections (warn + auto-add)
    let declared: HashSet<String> = doc.nodes.iter().map(|n| n.id.clone()).collect();
    let mut auto_added: HashSet<String> = HashSet::new();

    for conn in &doc.connections {
        for ref_id in [&conn.from, &conn.to] {
            if !declared.contains(ref_id.as_str()) && !auto_added.contains(ref_id.as_str()) {
                diagnostics.push(Diagnostic {
                    line: conn.span.line,
                    col: conn.span.col,
                    code: "W010".into(),
                    message: format!("Node '{}' used but not declared, treating as service", ref_id),
                    suggestion: Some(format!("Add: service {}", ref_id)),
                    severity: Severity::Warning,
                });
                // Add implicit node
                resolved.nodes.push(Node {
                    node_type: NodeType::Service,
                    id: ref_id.clone(),
                    label: None,
                    tags: Vec::new(),
                    span: conn.span.clone(),
                });
                auto_added.insert(ref_id.clone());
            }
        }
    }

    // 3. Check for undeclared nodes in groups
    fn check_group_refs(
        group: &Group,
        declared: &HashSet<String>,
        auto_added: &mut HashSet<String>,
        diagnostics: &mut Vec<Diagnostic>,
        resolved_nodes: &mut Vec<Node>,
    ) {
        for member in &group.members {
            match member {
                GroupMember::NodeRef(id) => {
                    if !declared.contains(id) && !auto_added.contains(id) {
                        diagnostics.push(Diagnostic {
                            line: group.span.line,
                            col: group.span.col,
                            code: "W011".into(),
                            message: format!("Node '{}' referenced in group '{}' but not declared", id, group.label),
                            suggestion: Some(format!("Add: service {}", id)),
                            severity: Severity::Warning,
                        });
                        resolved_nodes.push(Node {
                            node_type: NodeType::Service,
                            id: id.clone(),
                            label: None,
                            tags: Vec::new(),
                            span: group.span.clone(),
                        });
                        auto_added.insert(id.clone());
                    }
                }
                GroupMember::NodeRefList(ids) => {
                    for id in ids {
                        if !declared.contains(id) && !auto_added.contains(id) {
                            diagnostics.push(Diagnostic {
                                line: group.span.line,
                                col: group.span.col,
                                code: "W011".into(),
                                message: format!("Node '{}' referenced in group '{}' but not declared", id, group.label),
                                suggestion: Some(format!("Add: service {}", id)),
                                severity: Severity::Warning,
                            });
                            resolved_nodes.push(Node {
                                node_type: NodeType::Service,
                                id: id.clone(),
                                label: None,
                                tags: Vec::new(),
                                span: group.span.clone(),
                            });
                            auto_added.insert(id.clone());
                        }
                    }
                }
                GroupMember::Group(g) => {
                    check_group_refs(g, declared, auto_added, diagnostics, resolved_nodes);
                }
                _ => {}
            }
        }
    }

    for group in &doc.groups {
        check_group_refs(group, &declared, &mut auto_added, &mut diagnostics, &mut resolved.nodes);
    }

    // 4. Check for unused nodes (declared but never connected or grouped)
    let mut referenced: HashSet<&str> = HashSet::new();
    for conn in &doc.connections {
        referenced.insert(&conn.from);
        referenced.insert(&conn.to);
    }
    fn collect_group_member_ids<'a>(group: &'a Group, ids: &mut HashSet<&'a str>) {
        for member in &group.members {
            match member {
                GroupMember::NodeRef(id) => { ids.insert(id); }
                GroupMember::NodeRefList(list) => { for id in list { ids.insert(id); } }
                GroupMember::Node(n) => { ids.insert(&n.id); }
                GroupMember::Connection(c) => { ids.insert(&c.from); ids.insert(&c.to); }
                GroupMember::Group(g) => collect_group_member_ids(g, ids),
            }
        }
    }
    for group in &doc.groups {
        collect_group_member_ids(group, &mut referenced);
    }

    for node in &doc.nodes {
        if !referenced.contains(node.id.as_str()) {
            diagnostics.push(Diagnostic {
                line: node.span.line,
                col: node.span.col,
                code: "I001".into(),
                message: format!("Node '{}' is declared but never used in connections or groups", node.id),
                suggestion: None,
                severity: Severity::Info,
            });
        }
    }

    // 5. Check for self-connections
    for conn in &doc.connections {
        if conn.from == conn.to {
            diagnostics.push(Diagnostic {
                line: conn.span.line,
                col: conn.span.col,
                code: "W012".into(),
                message: format!("Self-connection on '{}' — is this intentional?", conn.from),
                suggestion: None,
                severity: Severity::Warning,
            });
        }
    }

    // Sort diagnostics by line, then severity
    diagnostics.sort_by(|a, b| {
        a.line.cmp(&b.line)
            .then(severity_order(&a.severity).cmp(&severity_order(&b.severity)))
    });

    let has_errors = diagnostics.iter().any(|d| d.severity == Severity::Error);
    let fixable = diagnostics.iter().all(|d| d.suggestion.is_some() || d.severity != Severity::Error);

    let result = ValidationResult {
        valid: !has_errors,
        errors: diagnostics,
        fixable,
        fixed_source: None, // TODO: implement auto-fix
    };

    (result, resolved)
}

fn severity_order(s: &Severity) -> u8 {
    match s {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}
