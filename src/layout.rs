/// Arc layout engine — simplified Sugiyama-style layered layout,
/// optimized for architecture diagrams (typically 5-40 nodes, hierarchical).
use crate::ast::*;
use std::collections::{HashMap, HashSet, VecDeque};

// ── Public types ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub groups: Vec<LayoutGroup>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub node_type: NodeType,
    pub label: String,
    pub display_label: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: String,
    pub to: String,
    pub points: Vec<(f64, f64)>,
    pub label: Option<String>,
    pub tags: Vec<String>,
    pub arrow_kind: ArrowKind,
}

#[derive(Debug, Clone)]
pub struct LayoutGroup {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub tags: Vec<String>,
    pub depth: usize,
    pub children: Vec<LayoutGroup>,
}

// ── Constants ────────────────────────────────────────────────────

const NODE_WIDTH: f64 = 170.0;
const NODE_HEIGHT: f64 = 72.0;
const NODE_HEIGHT_WITH_TAGS: f64 = 90.0;
const GROUP_PADDING: f64 = 28.0;
const GROUP_HEADER: f64 = 28.0;

// ── Layout computation ──────────────────────────────────────────

pub fn compute_layout(doc: &Document) -> LayoutResult {
    let direction = doc.direction();
    let spacing = doc.spacing();
    let layer_gap = spacing.layer_gap();
    let node_gap = spacing.node_gap();

    // Build adjacency info
    let node_ids: Vec<String> = doc.nodes.iter().map(|n| n.id.clone()).collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    for id in &node_ids {
        outgoing.entry(id.clone()).or_default();
        incoming.entry(id.clone()).or_default();
    }
    for conn in &doc.connections {
        if conn.arrow == ArrowKind::Blocked {
            continue;
        }
        outgoing
            .entry(conn.from.clone())
            .or_default()
            .push(conn.to.clone());
        if conn.arrow == ArrowKind::Bidirectional {
            outgoing
                .entry(conn.to.clone())
                .or_default()
                .push(conn.from.clone());
            incoming
                .entry(conn.from.clone())
                .or_default()
                .push(conn.to.clone());
        }
        incoming
            .entry(conn.to.clone())
            .or_default()
            .push(conn.from.clone());
    }

    // Step 1: Assign layers via BFS (longest path from sources)
    let mut layers: HashMap<String, usize> = HashMap::new();
    let sources: Vec<String> = node_ids
        .iter()
        .filter(|id| {
            incoming
                .get(id.as_str())
                .map(|v| v.is_empty())
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    // If no sources (cycle), pick the first node
    let seeds = if sources.is_empty() {
        node_ids.iter().take(1).cloned().collect::<Vec<_>>()
    } else {
        sources
    };

    // BFS to assign layers
    let mut queue: VecDeque<String> = VecDeque::new();
    for seed in &seeds {
        layers.insert(seed.clone(), 0);
        queue.push_back(seed.clone());
    }

    while let Some(node) = queue.pop_front() {
        let current_layer = *layers.get(&node).unwrap_or(&0);
        if let Some(neighbors) = outgoing.get(&node) {
            for next in neighbors {
                let new_layer = current_layer + 1;
                let existing = layers.get(next).copied().unwrap_or(0);
                if new_layer > existing || !layers.contains_key(next) {
                    layers.insert(next.clone(), new_layer);
                    queue.push_back(next.clone());
                }
            }
        }
    }

    // Ensure all nodes have a layer (disconnected nodes go to layer 0)
    for id in &node_ids {
        layers.entry(id.clone()).or_insert(0);
    }

    // Step 2: Group nodes by layer
    let max_layer = layers.values().copied().max().unwrap_or(0);
    let mut layer_nodes: Vec<Vec<String>> = vec![Vec::new(); max_layer + 1];
    for (id, layer) in &layers {
        layer_nodes[*layer].push(id.clone());
    }

    // Step 3: Order nodes within layers (barycenter heuristic)
    // Initialize with document order
    for layer in &mut layer_nodes {
        layer.sort_by_key(|id| node_ids.iter().position(|n| n == id).unwrap_or(0));
    }

    // Barycenter iterations
    for _iteration in 0..4 {
        for l in 1..=max_layer {
            let prev_layer = &layer_nodes[l - 1];
            let prev_positions: HashMap<String, f64> = prev_layer
                .iter()
                .enumerate()
                .map(|(i, id)| (id.clone(), i as f64))
                .collect();

            let mut barycenters: Vec<(String, f64)> = layer_nodes[l]
                .iter()
                .map(|id| {
                    let neighbors = incoming.get(id).cloned().unwrap_or_default();
                    let positions: Vec<f64> = neighbors
                        .iter()
                        .filter_map(|n| prev_positions.get(n).copied())
                        .collect();
                    let bc = if positions.is_empty() {
                        f64::MAX
                    } else {
                        positions.iter().sum::<f64>() / positions.len() as f64
                    };
                    (id.clone(), bc)
                })
                .collect();

            barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            layer_nodes[l] = barycenters.into_iter().map(|(id, _)| id).collect();
        }
    }

    // Step 4: Assign coordinates
    let node_map: HashMap<&str, &Node> = doc.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut layout_nodes: Vec<LayoutNode> = Vec::new();
    let mut node_positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();

    // Find max nodes in any layer for centering
    let max_nodes_in_layer = layer_nodes.iter().map(|l| l.len()).max().unwrap_or(1);

    for (layer_idx, nodes_in_layer) in layer_nodes.iter().enumerate() {
        let n = nodes_in_layer.len();
        for (pos_idx, node_id) in nodes_in_layer.iter().enumerate() {
            let node = node_map.get(node_id.as_str());
            let has_tags = node.map(|n| !n.tags.is_empty()).unwrap_or(false);
            let h = if has_tags {
                NODE_HEIGHT_WITH_TAGS
            } else {
                NODE_HEIGHT
            };

            // Center smaller layers
            let total_extent = n as f64 * h + (n as f64 - 1.0) * node_gap;
            let max_extent = max_nodes_in_layer as f64 * NODE_HEIGHT_WITH_TAGS
                + (max_nodes_in_layer as f64 - 1.0) * node_gap;
            let offset = (max_extent - total_extent) / 2.0;

            let (x, y) = match direction {
                Direction::Down => {
                    let x = offset + pos_idx as f64 * (NODE_WIDTH + node_gap);
                    let y = layer_idx as f64 * (NODE_HEIGHT_WITH_TAGS + layer_gap);
                    (x, y)
                }
                Direction::Right => {
                    let x = layer_idx as f64 * (NODE_WIDTH + layer_gap);
                    let y = offset + pos_idx as f64 * (NODE_HEIGHT_WITH_TAGS + node_gap);
                    (x, y)
                }
            };

            let display_label = node
                .map(|n| n.display_label().to_string())
                .unwrap_or_else(|| node_id.clone());

            let node_type = node.map(|n| n.node_type).unwrap_or(NodeType::Service);
            let tags = node.map(|n| n.tags.clone()).unwrap_or_default();

            layout_nodes.push(LayoutNode {
                id: node_id.clone(),
                x,
                y,
                width: NODE_WIDTH,
                height: h,
                node_type,
                label: node_id.clone(),
                display_label,
                tags,
            });

            node_positions.insert(node_id.clone(), (x, y, NODE_WIDTH, h));
        }
    }

    // Step 5: Route edges
    let mut layout_edges: Vec<LayoutEdge> = Vec::new();
    for conn in &doc.connections {
        if let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, th))) =
            (node_positions.get(&conn.from), node_positions.get(&conn.to))
        {
            let from_center = (fx + fw / 2.0, fy + fh / 2.0);
            let to_center = (tx + tw / 2.0, ty + th / 2.0);

            // Find connection points on node boundaries
            let from_point = edge_point(fx, fy, fw, fh, to_center.0, to_center.1);
            let to_point = edge_point(tx, ty, tw, th, from_center.0, from_center.1);

            layout_edges.push(LayoutEdge {
                from: conn.from.clone(),
                to: conn.to.clone(),
                points: vec![from_point, to_point],
                label: conn.label.clone(),
                tags: conn.tags.clone(),
                arrow_kind: conn.arrow,
            });
        }
    }

    // Step 6: Compute group bounds
    let layout_groups = compute_group_bounds(&doc.groups, &node_positions, 0);

    // Compute total canvas size
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for node in &layout_nodes {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    for group in &layout_groups {
        min_x = min_x.min(group.x);
        min_y = min_y.min(group.y);
        max_x = max_x.max(group.x + group.width);
        max_y = max_y.max(group.y + group.height);
    }

    // Normalize to positive coordinates with padding
    let pad = 40.0;
    let offset_x = -min_x + pad;
    let offset_y = -min_y + pad;

    for node in &mut layout_nodes {
        node.x += offset_x;
        node.y += offset_y;
    }
    for edge in &mut layout_edges {
        for point in &mut edge.points {
            point.0 += offset_x;
            point.1 += offset_y;
        }
    }
    fn offset_groups(groups: &mut Vec<LayoutGroup>, ox: f64, oy: f64) {
        for g in groups {
            g.x += ox;
            g.y += oy;
            offset_groups(&mut g.children, ox, oy);
        }
    }
    offset_groups(&mut Vec::new(), offset_x, offset_y);

    // Also update the layout_groups
    let mut layout_groups = layout_groups;
    fn offset_groups_in_place(groups: &mut [LayoutGroup], ox: f64, oy: f64) {
        for g in groups.iter_mut() {
            g.x += ox;
            g.y += oy;
            offset_groups_in_place(&mut g.children, ox, oy);
        }
    }
    offset_groups_in_place(&mut layout_groups, offset_x, offset_y);

    let width = (max_x - min_x) + pad * 2.0;
    let height = (max_y - min_y) + pad * 2.0;

    LayoutResult {
        nodes: layout_nodes,
        edges: layout_edges,
        groups: layout_groups,
        width: width.max(200.0),
        height: height.max(200.0),
    }
}

// ── Group bounds computation ────────────────────────────────────

fn compute_group_bounds(
    groups: &[Group],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    depth: usize,
) -> Vec<LayoutGroup> {
    let mut result = Vec::new();

    for group in groups {
        let mut member_ids: HashSet<String> = HashSet::new();
        let mut child_groups = Vec::new();

        collect_all_member_ids(group, &mut member_ids);

        // Recurse into sub-groups
        for member in &group.members {
            if let GroupMember::Group(sub) = member {
                let sub_bounds =
                    compute_group_bounds(std::slice::from_ref(sub), positions, depth + 1);
                child_groups.extend(sub_bounds);
            }
        }

        // Compute bounding box of all members
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut has_members = false;

        for id in &member_ids {
            if let Some(&(x, y, w, h)) = positions.get(id) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + w);
                max_y = max_y.max(y + h);
                has_members = true;
            }
        }

        // Also include child group bounds
        for cg in &child_groups {
            min_x = min_x.min(cg.x);
            min_y = min_y.min(cg.y);
            max_x = max_x.max(cg.x + cg.width);
            max_y = max_y.max(cg.y + cg.height);
            has_members = true;
        }

        if has_members {
            result.push(LayoutGroup {
                label: group.label.clone(),
                x: min_x - GROUP_PADDING,
                y: min_y - GROUP_PADDING - GROUP_HEADER,
                width: (max_x - min_x) + GROUP_PADDING * 2.0,
                height: (max_y - min_y) + GROUP_PADDING * 2.0 + GROUP_HEADER,
                tags: group.tags.clone(),
                depth,
                children: child_groups,
            });
        }
    }

    result
}

fn collect_all_member_ids(group: &Group, ids: &mut HashSet<String>) {
    for member in &group.members {
        match member {
            GroupMember::NodeRef(id) => {
                ids.insert(id.clone());
            }
            GroupMember::NodeRefList(list) => {
                ids.extend(list.iter().cloned());
            }
            GroupMember::Node(n) => {
                ids.insert(n.id.clone());
            }
            GroupMember::Connection(c) => {
                ids.insert(c.from.clone());
                ids.insert(c.to.clone());
            }
            GroupMember::Group(g) => {
                collect_all_member_ids(g, ids);
            }
        }
    }
}

// ── Edge point calculation ──────────────────────────────────────

/// Find the point on a rectangle boundary closest to the line from center to (tx, ty).
fn edge_point(rx: f64, ry: f64, rw: f64, rh: f64, tx: f64, ty: f64) -> (f64, f64) {
    let cx = rx + rw / 2.0;
    let cy = ry + rh / 2.0;
    let dx = tx - cx;
    let dy = ty - cy;

    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return (cx, cy);
    }

    let half_w = rw / 2.0;
    let half_h = rh / 2.0;

    // Find intersection with rectangle edges
    let scale_x = if dx.abs() > 0.001 {
        half_w / dx.abs()
    } else {
        f64::MAX
    };
    let scale_y = if dy.abs() > 0.001 {
        half_h / dy.abs()
    } else {
        f64::MAX
    };
    let scale = scale_x.min(scale_y);

    (cx + dx * scale, cy + dy * scale)
}
