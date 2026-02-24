/// Arc SVG renderer — produces clean, professional SVG output.
/// Hand-written SVG generation for zero dependencies and full control.

use crate::ast::{ArrowKind, NodeType};
use crate::layout::{LayoutEdge, LayoutGroup, LayoutNode, LayoutResult};
use crate::themes::Theme;

/// Render a layout result to an SVG string.
pub fn render_svg(layout: &LayoutResult, theme: &Theme) -> String {
    let mut svg = String::with_capacity(8192);
    let w = layout.width;
    let h = layout.height;

    // SVG header
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" font-family="{font}">"#,
        w = w.ceil() as i32,
        h = h.ceil() as i32,
        font = escape_xml(&theme.font.family),
    ));
    svg.push('\n');

    // Defs (arrow markers, filters)
    svg.push_str(&render_defs(theme));

    // Background
    svg.push_str(&format!(
        r#"  <rect width="100%" height="100%" fill="{}"/>"#,
        theme.background
    ));
    svg.push('\n');

    // Groups (backgrounds, rendered first / bottom layer)
    render_groups_recursive(&layout.groups, theme, &mut svg);

    // Edges (middle layer)
    for edge in &layout.edges {
        render_edge(edge, theme, &mut svg);
    }

    // Nodes (top layer)
    for node in &layout.nodes {
        render_node(node, theme, &mut svg);
    }

    svg.push_str("</svg>\n");
    svg
}

// ── Defs ─────────────────────────────────────────────────────────

fn render_defs(theme: &Theme) -> String {
    let cs = &theme.connection_style;
    let s = cs.arrow_size;
    format!(
        r#"  <defs>
    <marker id="arrow" viewBox="0 0 {s} {s}" refX="{s}" refY="{hs}" markerWidth="{s}" markerHeight="{s}" orient="auto-start-reverse">
      <path d="M0,0 L{s},{hs} L0,{s}" fill="{color}" />
    </marker>
    <marker id="arrow-dashed" viewBox="0 0 {s} {s}" refX="{s}" refY="{hs}" markerWidth="{s}" markerHeight="{s}" orient="auto-start-reverse">
      <path d="M0,0 L{s},{hs} L0,{s}" fill="{dashed_color}" />
    </marker>
    <marker id="arrow-blocked" viewBox="0 0 {s} {s}" refX="{s}" refY="{hs}" markerWidth="{s}" markerHeight="{s}" orient="auto-start-reverse">
      <path d="M0,0 L{s},{hs} L0,{s}" fill="{blocked_color}" />
    </marker>
    <filter id="shadow" x="-4%" y="-4%" width="108%" height="116%">
      <feDropShadow dx="0" dy="1" stdDeviation="2" flood-opacity="0.08" />
    </filter>
  </defs>
"#,
        s = s as i32,
        hs = (s / 2.0) as i32,
        color = cs.stroke,
        dashed_color = cs.dashed_stroke,
        blocked_color = cs.blocked_stroke,
    )
}

// ── Node rendering ──────────────────────────────────────────────

fn render_node(node: &LayoutNode, theme: &Theme, svg: &mut String) {
    let style = theme.node_style(&node.node_type);
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    let r = 10.0; // corner radius

    // Node group
    svg.push_str(&format!(r#"  <g class="node" data-id="{}">"#, escape_xml(&node.id)));
    svg.push('\n');

    // Shape based on node type
    match node.node_type {
        NodeType::Db | NodeType::Cache => {
            render_cylinder(svg, x, y, w, h, style, node.node_type == NodeType::Cache);
        }
        NodeType::Queue => {
            render_parallelogram(svg, x, y, w, h, style);
        }
        NodeType::User => {
            render_user_shape(svg, x, y, w, h, style);
        }
        NodeType::External => {
            render_cloud_rect(svg, x, y, w, h, style);
        }
        _ => {
            // Default: rounded rectangle
            svg.push_str(&format!(
                r#"    <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" ry="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1.5" filter="url(#shadow)"/>"#,
                x = x, y = y, w = w, h = h, r = r,
                fill = style.fill, stroke = style.stroke,
            ));
            svg.push('\n');
        }
    }

    // Type badge (top)
    let type_label = node.node_type.as_str().to_uppercase();
    let type_y = y + 20.0;
    svg.push_str(&format!(
        r#"    <text x="{x}" y="{y}" text-anchor="middle" fill="{color}" font-size="{size}" font-weight="600" letter-spacing="0.5">{text}</text>"#,
        x = x + w / 2.0,
        y = type_y,
        color = style.type_color,
        size = theme.font.node_type_size,
        text = escape_xml(&type_label),
    ));
    svg.push('\n');

    // Name label (center)
    let label_y = y + 42.0;
    let display = truncate_label(&node.display_label, 20);
    svg.push_str(&format!(
        r#"    <text x="{x}" y="{y}" text-anchor="middle" fill="{color}" font-size="{size}" font-weight="600">{text}</text>"#,
        x = x + w / 2.0,
        y = label_y,
        color = style.text_color,
        size = theme.font.node_label_size,
        text = escape_xml(&display),
    ));
    svg.push('\n');

    // Tags (bottom pills)
    if !node.tags.is_empty() {
        let tag_y = y + 60.0;
        let tag_text = node.tags.join(" · ");
        let display_tags = truncate_label(&tag_text, 24);
        svg.push_str(&format!(
            r#"    <text x="{x}" y="{y}" text-anchor="middle" fill="{color}" font-size="{size}">{text}</text>"#,
            x = x + w / 2.0,
            y = tag_y,
            color = style.type_color,
            size = theme.font.tag_size,
            text = escape_xml(&display_tags),
        ));
        svg.push('\n');
    }

    svg.push_str("  </g>\n");
}

fn render_cylinder(svg: &mut String, x: f64, y: f64, w: f64, h: f64, style: &crate::themes::NodeStyle, dashed: bool) {
    let ry = 8.0; // ellipse height for cylinder top/bottom
    let dash = if dashed { r#" stroke-dasharray="4,3""# } else { "" };

    // Body
    svg.push_str(&format!(
        r#"    <path d="M{x0},{y0} L{x0},{y1} Q{x0},{y2} {cx},{y2} Q{x1},{y2} {x1},{y1} L{x1},{y0}" fill="{fill}" stroke="{stroke}" stroke-width="1.5"{dash} filter="url(#shadow)"/>"#,
        x0 = x, y0 = y + ry, y1 = y + h - ry, y2 = y + h,
        x1 = x + w, cx = x + w / 2.0,
        fill = style.fill, stroke = style.stroke, dash = dash,
    ));
    svg.push('\n');

    // Top ellipse
    svg.push_str(&format!(
        r#"    <ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}" fill="{fill}" stroke="{stroke}" stroke-width="1.5"{dash}/>"#,
        cx = x + w / 2.0, cy = y + ry, rx = w / 2.0, ry = ry,
        fill = style.stroke, stroke = style.stroke, dash = dash,
    ));
    svg.push('\n');
}

fn render_parallelogram(svg: &mut String, x: f64, y: f64, w: f64, h: f64, style: &crate::themes::NodeStyle) {
    let skew = 15.0;
    svg.push_str(&format!(
        r#"    <path d="M{x0},{y0} L{x1},{y0} L{x2},{y1} L{x3},{y1} Z" fill="{fill}" stroke="{stroke}" stroke-width="1.5" filter="url(#shadow)"/>"#,
        x0 = x + skew, y0 = y, x1 = x + w, x2 = x + w - skew, y1 = y + h, x3 = x,
        fill = style.fill, stroke = style.stroke,
    ));
    svg.push('\n');
}

fn render_user_shape(svg: &mut String, x: f64, y: f64, w: f64, h: f64, style: &crate::themes::NodeStyle) {
    // Rounded rect with a small person icon area
    let r = 10.0;
    svg.push_str(&format!(
        r#"    <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" ry="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1.5" filter="url(#shadow)"/>"#,
        x = x, y = y, w = w, h = h, r = r,
        fill = style.fill, stroke = style.stroke,
    ));
    svg.push('\n');
}

fn render_cloud_rect(svg: &mut String, x: f64, y: f64, w: f64, h: f64, style: &crate::themes::NodeStyle) {
    let r = 10.0;
    svg.push_str(&format!(
        r#"    <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" ry="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1.5" stroke-dasharray="6,3" filter="url(#shadow)"/>"#,
        x = x, y = y, w = w, h = h, r = r,
        fill = style.fill, stroke = style.stroke,
    ));
    svg.push('\n');
}

// ── Edge rendering ──────────────────────────────────────────────

fn render_edge(edge: &LayoutEdge, theme: &Theme, svg: &mut String) {
    if edge.points.len() < 2 { return; }

    let cs = &theme.connection_style;
    let (stroke, marker, dash) = match edge.arrow_kind {
        ArrowKind::Solid => (&cs.stroke, "url(#arrow)", "".to_string()),
        ArrowKind::Dashed => (&cs.dashed_stroke, "url(#arrow-dashed)", r#" stroke-dasharray="6,4""#.to_string()),
        ArrowKind::Bidirectional => (&cs.stroke, "url(#arrow)", "".to_string()),
        ArrowKind::Blocked => (&cs.blocked_stroke, "url(#arrow-blocked)", "".to_string()),
    };

    let start = edge.points[0];
    let end = edge.points[edge.points.len() - 1];

    // Draw the path
    let mut path = format!("M{},{}", start.0.round(), start.1.round());
    for p in &edge.points[1..] {
        path.push_str(&format!(" L{},{}", p.0.round(), p.1.round()));
    }

    let marker_end = format!(r#" marker-end="{}""#, marker);
    let marker_start = if edge.arrow_kind == ArrowKind::Bidirectional {
        format!(r#" marker-start="{}""#, marker)
    } else {
        String::new()
    };

    svg.push_str(&format!(
        r#"  <path d="{path}" fill="none" stroke="{stroke}" stroke-width="{sw}"{dash}{me}{ms}/>"#,
        path = path,
        stroke = stroke,
        sw = cs.stroke_width,
        dash = dash,
        me = marker_end,
        ms = marker_start,
    ));
    svg.push('\n');

    // Label
    if let Some(ref label) = edge.label {
        let mid_x = (start.0 + end.0) / 2.0;
        let mid_y = (start.1 + end.1) / 2.0;
        let display = truncate_label(label, 24);

        // Background pill for label
        let text_width = display.len() as f64 * 6.5 + 12.0;
        let text_height = 18.0;
        svg.push_str(&format!(
            r#"  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="9" ry="9" fill="{bg}"/>"#,
            x = mid_x - text_width / 2.0,
            y = mid_y - text_height / 2.0,
            w = text_width,
            h = text_height,
            bg = cs.text_bg,
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="central" fill="{color}" font-size="{size}">{text}</text>"#,
            x = mid_x,
            y = mid_y,
            color = cs.text_color,
            size = cs.label_size,
            text = escape_xml(&display),
        ));
        svg.push('\n');
    }

    // Tags on edge
    if !edge.tags.is_empty() {
        let mid_x = (start.0 + end.0) / 2.0;
        let mid_y = (start.1 + end.1) / 2.0;
        let tag_y = mid_y + if edge.label.is_some() { 14.0 } else { 0.0 };
        let tag_text = edge.tags.join(", ");
        let display = truncate_label(&tag_text, 16);

        let tw = display.len() as f64 * 5.5 + 10.0;
        svg.push_str(&format!(
            r#"  <rect x="{x}" y="{y}" width="{w}" height="14" rx="7" ry="7" fill="{bg}"/>"#,
            x = mid_x - tw / 2.0,
            y = tag_y - 7.0,
            w = tw,
            bg = cs.tag_bg,
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="central" fill="{color}" font-size="9">{text}</text>"#,
            x = mid_x,
            y = tag_y,
            color = cs.tag_text,
            text = escape_xml(&display),
        ));
        svg.push('\n');
    }
}

// ── Group rendering ─────────────────────────────────────────────

fn render_groups_recursive(groups: &[LayoutGroup], theme: &Theme, svg: &mut String) {
    for group in groups {
        let gs = &theme.group_style;
        let fill = theme.group_fill(group.depth);
        let stroke = theme.group_stroke(group.depth);
        let dashed = group.tags.contains(&"dashed".to_string());

        let dash_attr = if dashed { r#" stroke-dasharray="6,4""# } else { "" };

        svg.push_str(&format!(
            r#"  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" ry="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1"{dash}/>"#,
            x = group.x, y = group.y, w = group.width, h = group.height,
            r = gs.corner_radius,
            fill = fill, stroke = stroke, dash = dash_attr,
        ));
        svg.push('\n');

        // Group label
        svg.push_str(&format!(
            r#"  <text x="{x}" y="{y}" fill="{color}" font-size="{size}" font-weight="500">{text}</text>"#,
            x = group.x + 12.0,
            y = group.y + 18.0,
            color = gs.text_color,
            size = gs.label_size,
            text = escape_xml(&group.label),
        ));
        svg.push('\n');

        // Recurse into children
        render_groups_recursive(&group.children, theme, svg);
    }
}

// ── Utilities ───────────────────────────────────────────────────

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

fn truncate_label(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
