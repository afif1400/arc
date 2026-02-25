# arc

A minimal, LLM-native architecture diagram language.

**670KB binary. Zero dependencies. One syntax pattern. Built for AI agents.**

```arc
service Auth "Auth Service" [Go, JWT]
service API "Core API" [Node.js]
db Postgres "Main DB" [v16]
cache Redis [cluster]

Auth -> API: "validate token"
API -> Postgres: "read/write"
API -> Redis: "cache lookup"
```

## Why arc?

Every diagram tool today was designed for humans. **arc is designed for LLMs.**

| Problem | How arc solves it |
|---------|-------------------|
| Mermaid needs Chrome to render | arc is a single 670KB binary |
| PlantUML needs Java | arc has zero runtime dependencies |
| Every tool has 5-12 different syntax patterns | arc has **one** pattern for everything |
| LLMs make syntax errors that break rendering | arc has a forgiving parser that warns and renders anyway |
| Error messages are cryptic | arc outputs machine-readable JSON with fix suggestions |
| Grammar specs are huge | arc's full grammar fits in **~120 tokens** |

## Install

### Homebrew (macOS/Linux)

```bash
brew install afif1400/tap/arc-lang
```

### Cargo (crates.io)

```bash
cargo install arc-lang
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/afif1400/arc/releases/latest) — available for macOS (Intel & Apple Silicon), Linux (x86 & ARM), and Windows.

### From source

```bash
git clone https://github.com/afif1400/arc.git
cd arc
cargo install --path .
```

## Quick Start

Create a file called `architecture.arc`:

```arc
@direction right
@theme light

user Client "Web Browser"
gateway LB "Load Balancer" [nginx]
service API "REST API" [Go, v2.1]
db Postgres "Main Database" [v16]
cache Redis "Session Cache" [cluster]

Client -> LB: "HTTPS"
LB -> API: "proxy"
API -> Postgres: "read/write"
API -> Redis: "session lookup"
```

Render it:

```bash
arc render architecture.arc -o diagram.svg
```

## CLI Reference

```bash
# Render to SVG
arc render input.arc -o output.svg

# Render with a specific theme
arc render input.arc -o output.svg --theme dark

# Pipe from stdin to stdout
echo 'service A\nA -> B: "hello"' | arc render > diagram.svg

# Validate (human-readable)
arc validate input.arc

# Validate (JSON for agents)
arc validate input.arc --format json

# Format/prettify
arc fmt input.arc
arc fmt input.arc --write  # in-place

# Print the full grammar spec (for LLM system prompts)
arc grammar

# Start MCP server
arc mcp
```

## Language Reference

### Nodes

Declare nodes with a **type** and **ID**. Optionally add a display label and tags.

```arc
service Auth                          # minimal
service Auth "Auth Service"           # with display label
service Auth "Auth Service" [Go, JWT] # with tags
```

**Built-in types** (each renders with a distinct shape and color):

| Type | Shape | Use for |
|------|-------|---------|
| `service` | Rounded rectangle | APIs, microservices, backend services |
| `db` | Cylinder | Databases (PostgreSQL, MySQL, MongoDB) |
| `cache` | Dashed cylinder | Caches (Redis, Memcached) |
| `queue` | Parallelogram | Message queues (Kafka, RabbitMQ, SQS) |
| `gateway` | Rounded rectangle (teal) | Load balancers, API gateways, proxies |
| `user` | Rounded rectangle (gray) | Users, clients, actors |
| `store` | Rounded rectangle (amber) | Object storage (S3, GCS, Azure Blob) |
| `fn` | Rounded rectangle (pink) | Serverless functions, lambdas |
| `worker` | Rounded rectangle (green) | Background workers, cron jobs |
| `external` | Dashed rectangle | Third-party services, external APIs |

### Connections

```arc
A -> B                    # solid arrow (synchronous)
A --> B                   # dashed arrow (async)
A <-> B                   # bidirectional
A -x B                    # blocked / deprecated

A -> B: "HTTPS POST"      # with label
A -> B: "events" [async]  # with label + tag badge
```

### Groups

Groups render as visual boundaries (VPCs, subnets, regions, clusters):

```arc
group "AWS us-east-1" {
  group "VPC" {
    group "Private Subnet" {
      API, Postgres, Redis
    }
    group "Public Subnet" {
      LB
    }
  }
}
```

Groups can be nested to any depth. Connections cross group boundaries freely.

### Directives

```arc
@direction right          # left-to-right layout (default: down)
@theme dark               # light | dark | blueprint | mono
@spacing compact          # compact | normal | wide
```

### Includes

Split large architectures across files:

```arc
include "./auth.arc"
include "./data-layer.arc"
```

## Themes

arc ships with 4 built-in themes:

| Theme | Best for |
|-------|----------|
| `light` | Documentation, presentations (default) |
| `dark` | Dark-mode docs, terminal-adjacent workflows |
| `blueprint` | Technical specs, engineering reviews |
| `mono` | Print-friendly, high contrast |

```bash
arc render input.arc --theme blueprint -o output.svg
```

## For AI Agents

arc is designed from the ground up for LLM generation.

### The Grammar Fits in a System Prompt

```bash
arc grammar  # prints ~120 token spec
```

Include this in your agent's system prompt and it can generate valid arc diagrams reliably.

### Forgiving Parser

LLMs make mistakes. arc handles them gracefully:

```bash
# No node declarations? Warns and renders anyway.
echo 'Auth -> DB: "query"' | arc render -o out.svg
# ⚠ Node 'Auth' used but not declared, treating as service

# Typos get suggestions:
echo 'servce Auth' | arc validate --format json
# → "Did you mean 'service'?"
```

### Machine-Readable Validation

```bash
arc validate broken.arc --format json
```

```json
{
  "valid": false,
  "errors": [
    {
      "line": 3,
      "col": 1,
      "code": "W010",
      "message": "Node 'Redis' used but not declared, treating as service",
      "suggestion": "Add: service Redis",
      "severity": "warning"
    }
  ],
  "fixable": true
}
```

An agent can: **generate → validate → fix → render** in a single loop.

### MCP Server

arc includes a built-in [Model Context Protocol](https://modelcontextprotocol.io/) server for integration with Claude, Cursor, Codex, and other AI tools:

```bash
arc mcp
```

Exposes these tools over JSON-RPC (stdio):

| Tool | Description |
|------|-------------|
| `arc_validate` | Validate syntax, return structured errors |
| `arc_render` | Render to SVG |
| `arc_format` | Format/prettify source |
| `arc_grammar` | Return the full grammar spec |

#### MCP Configuration

Add to your MCP client config (e.g. `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "arc": {
      "command": "arc",
      "args": ["mcp"]
    }
  }
}
```

## Full Example

```arc
@direction right
@theme light

# Clients
user Web "Web App"
user Mobile "Mobile App"
external Partner "Partner API"

# Edge
gateway LB "Load Balancer" [nginx]
service Auth "Auth Service" [Go, JWT]
service API "Core API" [Node.js]
service Search "Search Service" [Python]
service Notify "Notification Service" [Go]

# Data
db Main "PostgreSQL" [v16, primary]
db Analytics "ClickHouse" [analytics]
cache Session "Redis" [cluster]
queue Events "Kafka" [3 brokers]
store Files "S3 Bucket"

# Connections
Web -> LB: "HTTPS"
Mobile -> LB: "HTTPS"
Partner -> Auth: "OAuth2"
LB -> Auth: "authenticate"
LB -> API: "route request"
Auth -> Session: "session lookup"
API -> Main: "CRUD"
API -> Session: "cache reads"
API -> Events: "domain events" [async]
API -> Files: "file upload"
Events -> Search: "index updates" [async]
Events -> Notify: "send alerts" [async]
Events -> Analytics: "event stream" [async]

# Infrastructure boundaries
group "AWS us-east-1" {
  LB
  group "VPC" {
    group "Public Subnet" {
      LB
    }
    group "Private Subnet" {
      Auth, API, Search, Notify
      Main, Session, Events, Files
    }
  }
  Analytics
}
```

## Grammar Spec

The complete grammar — small enough to include in any LLM prompt:

```
NODES:
  TYPE ID ["Display Label"] [tag1, tag2]
  TYPE = service | db | cache | queue | gateway | user | store | fn | worker | external

CONNECTIONS:
  ID -> ID [: "label"] [tag]      # solid arrow
  ID --> ID [: "label"] [tag]     # dashed arrow (async)
  ID <-> ID [: "label"] [tag]     # bidirectional
  ID -x ID [: "label"]            # blocked/deprecated

GROUPS:
  group "Label" [tags] { ... }    # nestable

DIRECTIVES:
  @direction down|right
  @theme light|dark|blueprint|mono
  @spacing compact|normal|wide
```

## Project Structure

```
src/
  ast.rs         # Type system (10 node types, 4 arrow kinds, groups)
  lexer.rs       # Tokenizer with error recovery
  parser.rs      # Hand-rolled recursive descent parser
  validator.rs   # Semantic validation with fix suggestions
  layout.rs      # Sugiyama-style layered layout engine
  svg.rs         # SVG renderer (shapes, edges, groups, themes)
  themes.rs      # 4 built-in color themes
  fmt.rs         # Source code formatter
  mcp.rs         # MCP server (JSON-RPC over stdio)
  main.rs        # CLI (clap)
  lib.rs         # Public API
```

## Roadmap

- [ ] PNG output
- [ ] WASM build for browser rendering
- [ ] VS Code extension with live preview
- [ ] `arc serve --watch` dev server with hot reload
- [ ] Custom node types
- [ ] Notes/descriptions on nodes
- [ ] Interactive SVG (clickable, tooltips)
- [ ] Import from Mermaid/D2/PlantUML

## License

MIT
