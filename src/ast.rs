use serde::{Deserialize, Serialize};

// ── Source location ──────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

// ── Top-level document ───────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub directives: Vec<Directive>,
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
    pub groups: Vec<Group>,
    pub includes: Vec<Include>,
}

impl Document {
    /// Get the direction directive, defaulting to Down.
    pub fn direction(&self) -> Direction {
        self.directives
            .iter()
            .find_map(|d| {
                if let Directive::Direction(dir) = d {
                    Some(*dir)
                } else {
                    None
                }
            })
            .unwrap_or(Direction::Down)
    }

    /// Get the theme name, defaulting to "light".
    pub fn theme_name(&self) -> &str {
        self.directives
            .iter()
            .find_map(|d| {
                if let Directive::Theme(ref name) = d {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .unwrap_or("light")
    }

    /// Get the spacing directive, defaulting to Normal.
    pub fn spacing(&self) -> Spacing {
        self.directives
            .iter()
            .find_map(|d| {
                if let Directive::Spacing(s) = d {
                    Some(*s)
                } else {
                    None
                }
            })
            .unwrap_or(Spacing::Normal)
    }

    /// Find a node by ID.
    pub fn find_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Collect all node IDs referenced in connections but not declared.
    pub fn undeclared_node_ids(&self) -> Vec<String> {
        let declared: std::collections::HashSet<&str> =
            self.nodes.iter().map(|n| n.id.as_str()).collect();
        let mut referenced: std::collections::HashSet<String> = std::collections::HashSet::new();
        for conn in &self.connections {
            if !declared.contains(conn.from.as_str()) {
                referenced.insert(conn.from.clone());
            }
            if !declared.contains(conn.to.as_str()) {
                referenced.insert(conn.to.clone());
            }
        }
        // Also check group member refs
        fn collect_group_refs(
            group: &Group,
            declared: &std::collections::HashSet<&str>,
            refs: &mut std::collections::HashSet<String>,
        ) {
            for m in &group.members {
                match m {
                    GroupMember::NodeRef(id) => {
                        if !declared.contains(id.as_str()) {
                            refs.insert(id.clone());
                        }
                    }
                    GroupMember::Group(g) => collect_group_refs(g, declared, refs),
                    _ => {}
                }
            }
        }
        for g in &self.groups {
            collect_group_refs(g, &declared, &mut referenced);
        }
        let mut ids: Vec<String> = referenced.into_iter().collect();
        ids.sort();
        ids
    }
}

// ── Directives ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Directive {
    Direction(Direction),
    Theme(String),
    Spacing(Spacing),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Down,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spacing {
    Compact,
    Normal,
    Wide,
}

impl Spacing {
    pub fn layer_gap(&self) -> f64 {
        match self {
            Spacing::Compact => 100.0,
            Spacing::Normal => 160.0,
            Spacing::Wide => 220.0,
        }
    }

    pub fn node_gap(&self) -> f64 {
        match self {
            Spacing::Compact => 20.0,
            Spacing::Normal => 35.0,
            Spacing::Wide => 50.0,
        }
    }
}

// ── Nodes ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub id: String,
    pub label: Option<String>,
    pub tags: Vec<String>,
    pub span: Span,
}

impl Node {
    pub fn display_label(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Service,
    Db,
    Cache,
    Queue,
    Gateway,
    User,
    Store,
    Fn,
    Worker,
    External,
}

impl NodeType {
    pub fn from_str_fuzzy(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "service" | "svc" => Some(Self::Service),
            "db" | "database" | "datastore" => Some(Self::Db),
            "cache" => Some(Self::Cache),
            "queue" | "mq" | "broker" => Some(Self::Queue),
            "gateway" | "gw" | "proxy" | "lb" => Some(Self::Gateway),
            "user" | "client" | "actor" => Some(Self::User),
            "store" | "storage" | "bucket" | "blob" => Some(Self::Store),
            "fn" | "func" | "function" | "lambda" => Some(Self::Fn),
            "worker" | "job" | "cron" => Some(Self::Worker),
            "external" | "ext" | "cloud" | "third-party" => Some(Self::External),
            _ => None,
        }
    }

    /// Suggest the canonical type name for close misspellings.
    pub fn suggest(s: &str) -> Option<&'static str> {
        let s_lower = s.to_lowercase();
        let candidates = [
            "service", "db", "cache", "queue", "gateway", "user", "store", "fn", "worker",
            "external",
        ];
        candidates
            .iter()
            .find(|c| edit_distance(&s_lower, c) <= 2)
            .copied()
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Db => "db",
            Self::Cache => "cache",
            Self::Queue => "queue",
            Self::Gateway => "gateway",
            Self::User => "user",
            Self::Store => "store",
            Self::Fn => "fn",
            Self::Worker => "worker",
            Self::External => "external",
        }
    }
}

// ── Connections ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowKind {
    Solid,         // ->
    Dashed,        // -->
    Bidirectional, // <->
    Blocked,       // -x
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub from: String,
    pub arrow: ArrowKind,
    pub to: String,
    pub label: Option<String>,
    pub tags: Vec<String>,
    pub span: Span,
}

// ── Groups ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Group {
    pub label: String,
    pub tags: Vec<String>,
    pub members: Vec<GroupMember>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum GroupMember {
    NodeRef(String),
    NodeRefList(Vec<String>),
    Node(Node),
    Connection(Connection),
    Group(Group),
}

// ── Includes ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Include {
    pub path: String,
    pub span: Span,
}

// ── Diagnostics ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

// ── Utility: edit distance ───────────────────────────────────────

#[allow(clippy::needless_range_loop)]
fn edit_distance(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let m = a_bytes.len();
    let n = b_bytes.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}
