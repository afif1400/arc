/// Arc themes — opinionated color palettes for architecture diagrams.
/// No CSS, no custom colors. Just pick a theme and get a beautiful diagram.

use crate::ast::NodeType;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub background: String,
    pub canvas_padding: f64,
    pub node_styles: HashMap<NodeType, NodeStyle>,
    pub group_style: GroupStyle,
    pub connection_style: ConnectionStyle,
    pub font: FontConfig,
}

#[derive(Debug, Clone)]
pub struct NodeStyle {
    pub fill: String,
    pub stroke: String,
    pub text_color: String,
    pub type_color: String,
    pub tag_bg: String,
    pub tag_text: String,
}

#[derive(Debug, Clone)]
pub struct GroupStyle {
    pub fills: Vec<String>,  // Different fills for nesting depth
    pub strokes: Vec<String>,
    pub text_color: String,
    pub corner_radius: f64,
    pub padding: f64,
    pub label_size: f64,
}

#[derive(Debug, Clone)]
pub struct ConnectionStyle {
    pub stroke: String,
    pub stroke_width: f64,
    pub dashed_stroke: String,
    pub blocked_stroke: String,
    pub text_color: String,
    pub text_bg: String,
    pub arrow_size: f64,
    pub label_size: f64,
    pub tag_bg: String,
    pub tag_text: String,
}

#[derive(Debug, Clone)]
pub struct FontConfig {
    pub family: String,
    pub node_label_size: f64,
    pub node_type_size: f64,
    pub tag_size: f64,
}

impl Theme {
    pub fn node_style(&self, node_type: &NodeType) -> &NodeStyle {
        self.node_styles.get(node_type).unwrap_or_else(|| {
            self.node_styles.get(&NodeType::Service).unwrap()
        })
    }

    pub fn group_fill(&self, depth: usize) -> &str {
        let idx = depth.min(self.group_style.fills.len() - 1);
        &self.group_style.fills[idx]
    }

    pub fn group_stroke(&self, depth: usize) -> &str {
        let idx = depth.min(self.group_style.strokes.len() - 1);
        &self.group_style.strokes[idx]
    }
}

pub fn get_theme(name: &str) -> Theme {
    match name {
        "dark" => dark_theme(),
        "blueprint" => blueprint_theme(),
        "mono" => mono_theme(),
        "sketch" => light_theme(), // sketch uses light colors + different rendering
        _ => light_theme(),
    }
}

fn make_node_styles(entries: Vec<(NodeType, &str, &str, &str, &str, &str, &str)>) -> HashMap<NodeType, NodeStyle> {
    entries.into_iter().map(|(nt, fill, stroke, text, type_c, tag_bg, tag_text)| {
        (nt, NodeStyle {
            fill: fill.into(),
            stroke: stroke.into(),
            text_color: text.into(),
            type_color: type_c.into(),
            tag_bg: tag_bg.into(),
            tag_text: tag_text.into(),
        })
    }).collect()
}

fn light_theme() -> Theme {
    Theme {
        name: "light".into(),
        background: "#FAFBFC".into(),
        canvas_padding: 40.0,
        node_styles: make_node_styles(vec![
            (NodeType::Service,  "#4A90D9", "#2E6AB0", "#FFFFFF", "#B8D4F0", "#EBF2FA", "#2E6AB0"),
            (NodeType::Db,       "#E8913A", "#C47425", "#FFFFFF", "#F5D5B0", "#FDF0E2", "#C47425"),
            (NodeType::Cache,    "#50B88E", "#3A9171", "#FFFFFF", "#B8E4D0", "#E8F6F0", "#3A9171"),
            (NodeType::Queue,    "#8B6CC1", "#6B4FA0", "#FFFFFF", "#CFC0E5", "#F0EBF7", "#6B4FA0"),
            (NodeType::Gateway,  "#3AAFA9", "#2B8A85", "#FFFFFF", "#B0DCD9", "#E4F4F3", "#2B8A85"),
            (NodeType::User,     "#6B7B8D", "#4F5D6B", "#FFFFFF", "#BCC5CE", "#E8ECF0", "#4F5D6B"),
            (NodeType::Store,    "#D4884A", "#B06E35", "#FFFFFF", "#EDD0B3", "#FAF0E3", "#B06E35"),
            (NodeType::Fn,       "#C75C9B", "#A44580", "#FFFFFF", "#E7B8D3", "#F8EAF2", "#A44580"),
            (NodeType::Worker,   "#5A8F6A", "#437352", "#FFFFFF", "#B5D4BD", "#E6F0E9", "#437352"),
            (NodeType::External, "#95A5B6", "#6E8091", "#FFFFFF", "#CDD5DC", "#EDF0F3", "#6E8091"),
        ]),
        group_style: GroupStyle {
            fills: vec![
                "rgba(0,0,0,0.03)".into(),
                "rgba(0,0,0,0.02)".into(),
                "rgba(0,0,0,0.01)".into(),
            ],
            strokes: vec![
                "#CBD5E0".into(),
                "#E2E8F0".into(),
                "#EDF2F7".into(),
            ],
            text_color: "#4A5568".into(),
            corner_radius: 12.0,
            padding: 24.0,
            label_size: 13.0,
        },
        connection_style: ConnectionStyle {
            stroke: "#8896A4".into(),
            stroke_width: 1.5,
            dashed_stroke: "#A0AEC0".into(),
            blocked_stroke: "#E53E3E".into(),
            text_color: "#4A5568".into(),
            text_bg: "#FFFFFF".into(),
            arrow_size: 8.0,
            label_size: 11.0,
            tag_bg: "#EDF2F7".into(),
            tag_text: "#4A5568".into(),
        },
        font: FontConfig {
            family: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".into(),
            node_label_size: 14.0,
            node_type_size: 10.0,
            tag_size: 9.0,
        },
    }
}

fn dark_theme() -> Theme {
    Theme {
        name: "dark".into(),
        background: "#1A202C".into(),
        canvas_padding: 40.0,
        node_styles: make_node_styles(vec![
            (NodeType::Service,  "#2B6CB0", "#3182CE", "#E2E8F0", "#90CDF4", "#1A365D", "#90CDF4"),
            (NodeType::Db,       "#C05621", "#DD6B20", "#E2E8F0", "#FBD38D", "#652B19", "#FBD38D"),
            (NodeType::Cache,    "#276749", "#38A169", "#E2E8F0", "#9AE6B4", "#1C4532", "#9AE6B4"),
            (NodeType::Queue,    "#553C9A", "#805AD5", "#E2E8F0", "#D6BCFA", "#322659", "#D6BCFA"),
            (NodeType::Gateway,  "#234E52", "#319795", "#E2E8F0", "#81E6D9", "#1D4044", "#81E6D9"),
            (NodeType::User,     "#4A5568", "#718096", "#E2E8F0", "#CBD5E0", "#2D3748", "#CBD5E0"),
            (NodeType::Store,    "#9C4221", "#C05621", "#E2E8F0", "#FBD38D", "#652B19", "#FBD38D"),
            (NodeType::Fn,       "#702459", "#B83280", "#E2E8F0", "#FBB6CE", "#521B41", "#FBB6CE"),
            (NodeType::Worker,   "#22543D", "#38A169", "#E2E8F0", "#9AE6B4", "#1C4532", "#9AE6B4"),
            (NodeType::External, "#2D3748", "#4A5568", "#CBD5E0", "#A0AEC0", "#1A202C", "#A0AEC0"),
        ]),
        group_style: GroupStyle {
            fills: vec![
                "rgba(255,255,255,0.04)".into(),
                "rgba(255,255,255,0.03)".into(),
                "rgba(255,255,255,0.02)".into(),
            ],
            strokes: vec![
                "#4A5568".into(),
                "#2D3748".into(),
                "#1A202C".into(),
            ],
            text_color: "#A0AEC0".into(),
            corner_radius: 12.0,
            padding: 24.0,
            label_size: 13.0,
        },
        connection_style: ConnectionStyle {
            stroke: "#718096".into(),
            stroke_width: 1.5,
            dashed_stroke: "#4A5568".into(),
            blocked_stroke: "#FC8181".into(),
            text_color: "#A0AEC0".into(),
            text_bg: "#2D3748".into(),
            arrow_size: 8.0,
            label_size: 11.0,
            tag_bg: "#2D3748".into(),
            tag_text: "#A0AEC0".into(),
        },
        font: FontConfig {
            family: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".into(),
            node_label_size: 14.0,
            node_type_size: 10.0,
            tag_size: 9.0,
        },
    }
}

fn blueprint_theme() -> Theme {
    Theme {
        name: "blueprint".into(),
        background: "#0D2137".into(),
        canvas_padding: 40.0,
        node_styles: make_node_styles(vec![
            (NodeType::Service,  "#133D6B", "#2980B9", "#D6EAF8", "#85C1E9", "#0B2545", "#85C1E9"),
            (NodeType::Db,       "#1B4D3E", "#27AE60", "#D5F5E3", "#82E0AA", "#0B3D2E", "#82E0AA"),
            (NodeType::Cache,    "#4A235A", "#8E44AD", "#E8DAEF", "#BB8FCE", "#2C1338", "#BB8FCE"),
            (NodeType::Queue,    "#1B4F72", "#2E86C1", "#D6EAF8", "#85C1E9", "#0B3D5C", "#85C1E9"),
            (NodeType::Gateway,  "#0E4D4D", "#17A589", "#D1F2EB", "#76D7C4", "#0A3D3D", "#76D7C4"),
            (NodeType::User,     "#1C2833", "#5D6D7E", "#D6DBDF", "#AEB6BF", "#0E1A25", "#AEB6BF"),
            (NodeType::Store,    "#1B3A4B", "#2E86C1", "#D6EAF8", "#85C1E9", "#0B2A3B", "#85C1E9"),
            (NodeType::Fn,       "#4A235A", "#AF7AC5", "#E8DAEF", "#D2B4DE", "#2C1338", "#D2B4DE"),
            (NodeType::Worker,   "#1B4D3E", "#2ECC71", "#D5F5E3", "#82E0AA", "#0B3D2E", "#82E0AA"),
            (NodeType::External, "#1C2833", "#5D6D7E", "#D6DBDF", "#AEB6BF", "#0E1A25", "#AEB6BF"),
        ]),
        group_style: GroupStyle {
            fills: vec![
                "rgba(41,128,185,0.08)".into(),
                "rgba(41,128,185,0.05)".into(),
                "rgba(41,128,185,0.03)".into(),
            ],
            strokes: vec![
                "#2980B9".into(),
                "#1F6FA3".into(),
                "#155A8A".into(),
            ],
            text_color: "#85C1E9".into(),
            corner_radius: 4.0,
            padding: 24.0,
            label_size: 13.0,
        },
        connection_style: ConnectionStyle {
            stroke: "#5DADE2".into(),
            stroke_width: 1.0,
            dashed_stroke: "#3498DB".into(),
            blocked_stroke: "#E74C3C".into(),
            text_color: "#85C1E9".into(),
            text_bg: "#0D2137".into(),
            arrow_size: 8.0,
            label_size: 11.0,
            tag_bg: "#133D6B".into(),
            tag_text: "#85C1E9".into(),
        },
        font: FontConfig {
            family: "'SF Mono', 'Fira Code', 'Consolas', monospace".into(),
            node_label_size: 13.0,
            node_type_size: 9.0,
            tag_size: 9.0,
        },
    }
}

fn mono_theme() -> Theme {
    Theme {
        name: "mono".into(),
        background: "#FFFFFF".into(),
        canvas_padding: 40.0,
        node_styles: make_node_styles(vec![
            (NodeType::Service,  "#F7F7F7", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::Db,       "#F0F0F0", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::Cache,    "#F7F7F7", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::Queue,    "#F0F0F0", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::Gateway,  "#F7F7F7", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::User,     "#F0F0F0", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::Store,    "#F7F7F7", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::Fn,       "#F0F0F0", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::Worker,   "#F7F7F7", "#333333", "#333333", "#888888", "#EEEEEE", "#555555"),
            (NodeType::External, "#F0F0F0", "#555555", "#333333", "#888888", "#EEEEEE", "#555555"),
        ]),
        group_style: GroupStyle {
            fills: vec![
                "rgba(0,0,0,0.02)".into(),
                "rgba(0,0,0,0.01)".into(),
                "rgba(0,0,0,0.005)".into(),
            ],
            strokes: vec![
                "#CCCCCC".into(),
                "#DDDDDD".into(),
                "#EEEEEE".into(),
            ],
            text_color: "#666666".into(),
            corner_radius: 8.0,
            padding: 24.0,
            label_size: 13.0,
        },
        connection_style: ConnectionStyle {
            stroke: "#999999".into(),
            stroke_width: 1.0,
            dashed_stroke: "#BBBBBB".into(),
            blocked_stroke: "#CC3333".into(),
            text_color: "#666666".into(),
            text_bg: "#FFFFFF".into(),
            arrow_size: 8.0,
            label_size: 11.0,
            tag_bg: "#F0F0F0".into(),
            tag_text: "#666666".into(),
        },
        font: FontConfig {
            family: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".into(),
            node_label_size: 14.0,
            node_type_size: 10.0,
            tag_size: 9.0,
        },
    }
}
