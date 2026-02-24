//! Arc formatter — pretty-prints .arc files with consistent style.

use crate::ast::*;

pub fn format_document(doc: &Document) -> String {
    let mut out = String::new();

    // Directives first
    for dir in &doc.directives {
        match dir {
            Directive::Direction(d) => {
                let val = match d {
                    Direction::Down => "down",
                    Direction::Right => "right",
                };
                out.push_str(&format!("@direction {}\n", val));
            }
            Directive::Theme(t) => {
                out.push_str(&format!("@theme {}\n", t));
            }
            Directive::Spacing(s) => {
                let val = match s {
                    Spacing::Compact => "compact",
                    Spacing::Normal => "normal",
                    Spacing::Wide => "wide",
                };
                out.push_str(&format!("@spacing {}\n", val));
            }
        }
    }
    if !doc.directives.is_empty() {
        out.push('\n');
    }

    // Includes
    for inc in &doc.includes {
        out.push_str(&format!("include \"{}\"\n", inc.path));
    }
    if !doc.includes.is_empty() {
        out.push('\n');
    }

    // Nodes
    if !doc.nodes.is_empty() {
        for node in &doc.nodes {
            format_node(node, &mut out, 0);
        }
        out.push('\n');
    }

    // Connections
    if !doc.connections.is_empty() {
        for conn in &doc.connections {
            format_connection(conn, &mut out, 0);
        }
        out.push('\n');
    }

    // Groups
    for group in &doc.groups {
        format_group(group, &mut out, 0);
        out.push('\n');
    }

    // Trim trailing whitespace
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

fn format_node(node: &Node, out: &mut String, indent: usize) {
    let pad = "  ".repeat(indent);
    out.push_str(&pad);
    out.push_str(node.node_type.as_str());
    out.push(' ');
    out.push_str(&node.id);

    if let Some(ref label) = node.label {
        out.push_str(&format!(" \"{}\"", label));
    }

    format_tags(&node.tags, out);
    out.push('\n');
}

fn format_connection(conn: &Connection, out: &mut String, indent: usize) {
    let pad = "  ".repeat(indent);
    let arrow = match conn.arrow {
        ArrowKind::Solid => "->",
        ArrowKind::Dashed => "-->",
        ArrowKind::Bidirectional => "<->",
        ArrowKind::Blocked => "-x",
    };

    out.push_str(&format!("{}{} {} {}", pad, conn.from, arrow, conn.to));

    if let Some(ref label) = conn.label {
        out.push_str(&format!(": \"{}\"", label));
    }

    format_tags(&conn.tags, out);
    out.push('\n');
}

fn format_group(group: &Group, out: &mut String, indent: usize) {
    let pad = "  ".repeat(indent);
    out.push_str(&format!("{}group \"{}\"", pad, group.label));
    format_tags(&group.tags, out);
    out.push_str(" {\n");

    for member in &group.members {
        match member {
            GroupMember::NodeRef(id) => {
                out.push_str(&format!("{}  {}\n", pad, id));
            }
            GroupMember::NodeRefList(ids) => {
                out.push_str(&format!("{}  {}\n", pad, ids.join(", ")));
            }
            GroupMember::Node(node) => {
                format_node(node, out, indent + 1);
            }
            GroupMember::Connection(conn) => {
                format_connection(conn, out, indent + 1);
            }
            GroupMember::Group(sub) => {
                format_group(sub, out, indent + 1);
            }
        }
    }

    out.push_str(&format!("{}}}\n", pad));
}

fn format_tags(tags: &[String], out: &mut String) {
    if !tags.is_empty() {
        out.push_str(&format!(" [{}]", tags.join(", ")));
    }
}
