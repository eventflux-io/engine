# EventFlux Low-Code Studio

> Visual drag-and-drop editor for creating EventFlux streaming applications

## Current Status

### Phase 1: Foundation - COMPLETE

| Feature | Status | Notes |
|---------|--------|-------|
| VS Code extension scaffolding | Done | `studio/` directory |
| React webview with layout | Done | Palette, Canvas, Properties, SQL Editor |
| Element palette with drag-and-drop | Done | All element types |
| React Flow canvas | Done | Custom node types, connection handling |
| Stream element | Done | Schema editor, attributes |
| Source/Sink elements | Done | Dynamic schema-based configuration |
| Filter, Projection, Aggregation | Done | Expression builder, attribute autocomplete |
| Window element | Done | All 9 window types |
| Join element | Done | 4 join types |
| Pattern element | Done | Basic pattern builder |
| Query generation (Visual → SQL) | Done | Multi-pass support |
| File save/load | Done | `.eventflux.studio` format |
| Schema generation system | Done | Build-time schema from Rust |
| CI/CD pipeline | Done | GitHub Actions workflow |

### Phase 2: In Progress

| Feature | Status | Notes |
|---------|--------|-------|
| SQL → Visual parsing | Partial | Basic parsing exists |
| Bidirectional sync | Pending | |
| Validation messages | Pending | |

## Schema Generation System

The Studio uses a **build-time schema generation** approach:

```
┌─────────────────────┐      cargo run --bin      ┌──────────────────────┐
│  EventFlux Engine   │  ─────────────────────►   │  eventflux-schema.json│
│  (Rust Factories)   │     generate_schema       │  (JSON Schema)        │
└─────────────────────┘                           └──────────────────────┘
                                                            │
                                                            ▼
                                                  ┌──────────────────────┐
                                                  │  Studio Webview      │
                                                  │  (Dynamic Forms)     │
                                                  └──────────────────────┘
```

### Generated Schema Contents

| Category | Count | Used For |
|----------|-------|----------|
| Sources | 3 | Source connector type dropdown |
| Sinks | 3 | Sink connector type dropdown |
| Mappers | 3 | Format selection (future) |
| Windows | 10 | Window type selection (future) |
| Aggregators | 8 | Aggregation function picker (future) |
| Functions | 45 | Expression builder autocomplete (future) |
| Collection Aggregators | 6 | Pattern aggregations (future) |
| Tables | 3 | Table extension selection (future) |

### Schema Utility Functions

Located in `studio/webview/src/schemas/index.ts`:

#### Currently Used (Phase 1)

| Function | Purpose | Used In |
|----------|---------|---------|
| `getSourceTypes()` | Returns list of available source connectors | Source element type dropdown |
| `getSinkTypes()` | Returns list of available sink connectors | Sink element type dropdown |
| `getSourceSchema(type)` | Gets metadata for a source connector | Source property editor |
| `getSinkSchema(type)` | Gets metadata for a sink connector | Sink property editor |
| `getConnectorParameters(schema)` | Extracts required + optional params | Dynamic form field generation |
| `formatParameterName(key)` | Formats param key for display | Form labels ("rabbitmq.host" → "Host") |

#### Reserved for Future Use (Phase 3)

| Function | Future Purpose | Integration Point |
|----------|----------------|-------------------|
| `getMapperTypes()` | Format picker dropdown (json, csv, bytes) | Source/Sink config panel - allows selecting data format for connectors that support multiple formats |
| `getWindowTypes()` | Window type dropdown with descriptions | Window element editor - replaces hardcoded window types with schema-driven list including descriptions |
| `getAggregatorTypes()` | Aggregation function picker | Aggregation element - provides dropdown of available aggregators (SUM, AVG, COUNT, MIN, MAX, etc.) with arity info |
| `getFunctionNames()` | Expression builder autocomplete | Filter/Projection expression input - autocomplete suggestions when typing function names (abs, concat, upper, etc.) |
| `getTableExtensions()` | Table extension dropdown | Table element editor - allows selecting table backend (inMemory, cache, jdbc) |
| `getMapperSchema(type)` | Mapper parameter configuration | Source/Sink config - dynamic form for mapper-specific params (e.g., csv.delimiter, json.date-format) |

### Regenerating the Schema

When new connectors or functions are added to the EventFlux engine:

```bash
cargo run --bin generate_schema
```

This updates `studio/webview/src/schemas/eventflux-schema.json`.

## CI/CD Pipeline

GitHub Actions workflow at `.github/workflows/studio.yml`:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ generate-schema │ ──► │      build      │ ──► │    publish      │
│   (Rust)        │     │   (Node.js)     │     │ (VS Code Mkt)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### Triggers

| Event | Action |
|-------|--------|
| Push to `main` (studio paths) | Build & test |
| Pull request (studio paths) | Build & test |
| Release published | Build & publish to Marketplace |
| Manual dispatch | Optional publish |

### Required Secrets

| Secret | Purpose |
|--------|---------|
| `VSCE_PAT` | VS Code Marketplace publish token |

Note: The workflow does NOT need VSIX cleanup (`rm -f *.vsix`) because each GitHub Actions run starts with a fresh checkout - no pre-existing artifacts exist.

## Directory Structure

```
studio/
├── package.json                    # VS Code extension manifest
├── .vscodeignore                   # Files excluded from VSIX
├── LICENSE                         # Apache 2.0
├── README.md                       # Extension readme (for Marketplace)
├── icons/
│   ├── logo.png                    # Extension icon (128KB)
│   ├── studio-dark.svg             # File icon (dark theme)
│   └── studio-light.svg            # File icon (light theme)
├── src/
│   ├── extension.ts                # Extension entry point
│   └── providers/
│       └── studioEditorProvider.ts # Custom editor provider
├── templates/
│   └── stock-analysis.json         # Example template
├── dist/                           # Compiled extension (esbuild)
└── webview/
    ├── package.json                # React app dependencies
    ├── vite.config.ts              # Vite build config
    ├── src/
    │   ├── App.tsx                 # React root
    │   ├── main.tsx                # Entry point
    │   ├── components/
    │   │   ├── Canvas/             # React Flow canvas
    │   │   ├── Palette/            # Element palette
    │   │   ├── Properties/         # Properties panel (1900+ lines)
    │   │   ├── SQLEditor/          # Monaco SQL editor
    │   │   ├── Toolbar/            # Action toolbar
    │   │   └── ConfigPanel/        # Config panel
    │   ├── elements/
    │   │   └── nodeTypes.tsx       # React Flow node definitions
    │   ├── schemas/
    │   │   ├── index.ts            # Schema types & helpers
    │   │   └── eventflux-schema.json  # Generated schema
    │   ├── stores/
    │   │   └── applicationStore.ts # Zustand state (970+ lines)
    │   ├── types/
    │   │   └── index.ts            # TypeScript types
    │   └── utils/
    │       ├── sqlParser.ts        # SQL parsing utilities
    │       ├── connectionRules.ts  # Connection validation
    │       └── vscode.ts           # VS Code API wrapper
    └── dist/                       # Compiled webview (Vite)
```

## Technology Stack

| Component | Technology | Version |
|-----------|------------|---------|
| Extension Host | TypeScript | 5.x |
| Webview Framework | React | 18.x |
| Canvas Library | React Flow | 11.x |
| SQL Editor | Monaco Editor | (via @monaco-editor/react) |
| State Management | Zustand | 4.x |
| Styling | Tailwind CSS | 3.x |
| Build (Extension) | esbuild | - |
| Build (Webview) | Vite | 5.x |

## Deployment Options

### Primary: VS Code Extension (Current)
- Webview-based visual canvas inside VS Code
- Direct `.eventflux.studio` file integration
- Works in both desktop and browser-based VS Code

### Secondary: Tauri Desktop App (Future)
- Rust backend with embedded EventFlux engine
- Cross-platform standalone application

### Tertiary: Docker Web App (Future)
- Standalone web application
- Connects to remote EventFlux engine

## Implementation Phases

### Phase 1: Foundation (MVP) - COMPLETE
- [x] VS Code extension scaffolding
- [x] React webview with basic layout
- [x] Element palette with drag-and-drop
- [x] Basic canvas with React Flow
- [x] All element types (Stream, Source, Sink, Window, Filter, etc.)
- [x] Query generation (Visual → SQL)
- [x] File save/load
- [x] Schema generation from Rust
- [x] CI/CD pipeline

### Phase 2: Core Processing - IN PROGRESS
- [ ] Import/Export `.eventflux` files (see "File Format Strategy" section)
- [ ] Real-time validation messages
- [ ] Monaco SQL editor improvements
- [ ] Error markers in visual elements
- [ ] **Attribute schema propagation** - Track available attributes through the flow (see "Schema Propagation" section below)

### Phase 3: Advanced Features
- [ ] Pattern visual builder (advanced)
- [ ] Expression builder autocomplete (using `getFunctionNames()`)
- [ ] Window type descriptions (using `getWindowTypes()`)
- [ ] Format picker for connectors (using `getMapperTypes()`)

### Phase 4: Execution & Simulation
- [ ] EventFlux engine client
- [ ] Live deployment and execution
- [ ] Test event sender
- [ ] Output event viewer
- [ ] Simulation engine
- [ ] Mock data generators

### Phase 5: Polish
- [ ] Auto-layout algorithm (Dagre)
- [ ] Undo/redo system
- [ ] Copy/paste elements
- [ ] Keyboard shortcuts
- [ ] Theme support (light/dark)

## File Format Strategy

### Why Two File Extensions?

| Extension | Format | Purpose |
|-----------|--------|---------|
| `.eventflux` | SQL | Production queries, runnable by EventFlux engine |
| `.eventflux.studio` | JSON | Visual Studio projects (nodes, edges, positions, properties) |

**The file extension is a promise to the user:**

- `.eventflux` promises: "I am valid EventFlux SQL. Run me with the engine."
- `.eventflux.studio` promises: "I am a Studio project. Open me in Studio."

### The Problem with a Single Extension

The visual editor cannot yet represent 100% of the EventFlux SQL grammar:

| Feature | Visual Support | Risk if Editing `.eventflux` Directly |
|---------|----------------|---------------------------------------|
| Basic streams | ✅ Full | Safe |
| Filters, projections | ✅ Full | Safe |
| Windows (all 9 types) | ✅ Full | Safe |
| Joins | ✅ Full | Safe |
| WITH clauses | ❌ Not yet | **Would be lost on save** |
| Complex patterns | ⚠️ Partial | **Features may be lost** |
| Advanced expressions | ⚠️ Partial | **May be simplified** |

**Data loss is unacceptable.** If a user opens a production `.eventflux` file with WITH clauses, edits something visually, and saves—those WITH clauses would silently disappear.

### Current Approach (Phase 1-2)

```
┌─────────────────┐                         ┌─────────────────────┐
│  .eventflux    │  ──── Import ─────────► │  .eventflux.studio  │
│  (SQL, full)   │                          │  (JSON, visual)     │
│                │  ◄──── Export ─────────  │                     │
└─────────────────┘         │               └─────────────────────┘
                            │
                            ▼
                  ⚠️ Warning shown if
                  features will be lost
```

**Import workflow:**
1. File → Import from `.eventflux`
2. Parser identifies unsupported features
3. Warning dialog: "This file uses features not yet supported: WITH clause (line 5). Import anyway?"
4. User decides with full information

**Export workflow:**
1. File → Export to `.eventflux`
2. Generates clean SQL from visual representation
3. No data loss because the visual canvas is the source of truth

### Future: Graceful Degradation (Phase 3+)

Once Studio matures, it will support opening `.eventflux` files directly with "SQL Block" nodes:

```
┌──────────────────────────────────────────────────────┐
│  Canvas                                              │
│                                                      │
│  ┌─────────┐     ┌─────────┐     ┌─────────┐        │
│  │ Stream  │────►│ Filter  │────►│ Output  │        │
│  └─────────┘     └─────────┘     └─────────┘        │
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │ ⚠️ SQL Block (preserved, read-only)            │ │
│  │ ┌────────────────────────────────────────────┐ │ │
│  │ │ WITH (                                     │ │ │
│  │ │   rabbitmq.host = 'localhost',             │ │ │
│  │ │   rabbitmq.port = 5672                     │ │ │
│  │ │ )                                          │ │ │
│  │ └────────────────────────────────────────────┘ │ │
│  │ This feature is not yet editable visually     │ │
│  └────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

- Unsupported SQL constructs become "SQL Block" nodes
- These blocks are **preserved exactly** on save (lossless round-trip)
- Users can edit supported parts visually
- As Studio grows, fewer SQL Blocks needed

### Roadmap

| Phase | File Strategy |
|-------|---------------|
| Phase 1-2 | Keep `.eventflux.studio` separate, add Import/Export with warnings |
| Phase 3 | Implement SQL Block nodes for graceful degradation |
| Phase 4+ | Consider `.eventflux` + `.eventflux.layout` sidecar when Studio covers 90%+ of grammar |

### Benefits of Current Approach

1. **No silent data loss** - Users explicitly import/export with warnings
2. **Clear expectations** - `.eventflux.studio` is obviously a Studio file
3. **Safe experimentation** - Can't accidentally break production queries
4. **Separate concerns** - Visual layout (JSON) vs executable code (SQL)

## Schema Propagation (TODO - Phase 2)

A critical feature for the visual editor is understanding what attributes are available at each point in the processing flow. This enables:
- **Attribute autocomplete** in Filter, Projection, Aggregation expressions
- **Validation** of attribute references
- **Type checking** for expressions

### The Problem

```
┌─────────┐      ┌─────────┐      ┌────────────┐      ┌────────┐
│ Stream  │ ──►  │ Filter  │ ──►  │ Projection │ ──►  │ Output │
│ (a,b,c) │      │ a > 10  │      │ SELECT a,  │      │        │
│         │      │         │      │ b+c AS sum │      │        │
└─────────┘      └─────────┘      └────────────┘      └────────┘
     │                │                  │                 │
     ▼                ▼                  ▼                 ▼
  {a,b,c}          {a,b,c}            {a,sum}          {a,sum}
```

Each element needs to know:
1. **Upstream schema** - What attributes are available from preceding elements
2. **Output schema** - What attributes this element produces for downstream elements

### Scenarios to Handle

| Element | Input Schema | Output Schema | Notes |
|---------|--------------|---------------|-------|
| Stream | (defined) | Same as definition | Source of schema |
| Filter | Upstream | Same as upstream | Filter doesn't change schema |
| Projection | Upstream | Selected columns + aliases | Schema changes! |
| Aggregation | Upstream | Group-by cols + aggregated cols | Schema changes! |
| Window | Upstream | Same as upstream | Adds window context |
| Join | Left + Right | Combined (with prefixes) | Merges two schemas |
| Pattern | Multiple streams | Selected from matched events | Complex mapping |

### Implementation Approach

```typescript
// In applicationStore.ts
function getUpstreamSchema(elementId: string): Attribute[] {
  // 1. Find all upstream elements via connections
  // 2. For each upstream element, get its output schema
  // 3. Merge/transform based on element type
  // 4. Return available attributes with types
}

function getOutputSchema(element: Element): Attribute[] {
  const upstream = getUpstreamSchema(element.id);

  switch (element.type) {
    case 'filter':
      return upstream; // Pass-through
    case 'projection':
      return element.properties.selectList.map(/*...*/);
    case 'aggregation':
      return [...groupByAttrs, ...aggregatedAttrs];
    // ...
  }
}
```

### Current State

Basic `getUpstreamSchema()` exists but needs enhancement for:
- [ ] Projection output schema calculation
- [ ] Aggregation output schema calculation
- [ ] Join schema merging with alias handling
- [ ] Pattern schema extraction from matched events
- [ ] Type inference for expressions (e.g., `a + b` → DOUBLE if both are numeric)

## Development Guide

### Prerequisites

- Node.js 18+
- VS Code
- Rust toolchain (for schema generation)

### Building the Extension

```bash
# From repository root
cd studio

# Install dependencies
npm install
cd webview && npm install && cd ..

# Build everything
npm run build

# Or build separately
npm run build:extension  # Extension host (esbuild)
npm run build:webview    # React webview (Vite)
```

### Running in Development Mode

1. **Open the `studio/` folder in VS Code**
   ```bash
   code studio/
   ```

2. **Press `F5`** to launch the Extension Development Host
   - This opens a new VS Code window with the extension loaded
   - Changes to extension code require restart (`Ctrl+Shift+F5`)

3. **Open a `.eventflux.studio` file** in the dev host to see the visual editor

### Debugging

#### Extension Host (TypeScript)
- Set breakpoints in `src/extension.ts` or `src/providers/`
- Debug output appears in the **Debug Console** panel
- Use `console.log()` statements - they appear in Debug Console

#### Webview (React)
- In the Extension Development Host, open **Developer Tools**:
  - `Cmd+Shift+P` (Mac) / `Ctrl+Shift+P` (Windows)
  - Type "Developer: Open Webview Developer Tools"
- This opens Chrome DevTools for the webview
- Use React DevTools extension for component inspection
- `console.log()` in React code appears here

#### Common Debug Commands

| Action | Shortcut |
|--------|----------|
| Start debugging | `F5` |
| Restart debugging | `Ctrl+Shift+F5` |
| Stop debugging | `Shift+F5` |
| Open command palette | `Ctrl+Shift+P` |
| Open webview devtools | Command: "Developer: Open Webview Developer Tools" |
| Reload webview | Command: "Developer: Reload Webview" |

### Watch Mode (Development)

```bash
# Terminal 1: Watch extension
cd studio
npm run watch:extension

# Terminal 2: Watch webview
cd studio/webview
npm run dev
```

Note: Webview hot-reload doesn't work in VS Code extension context. After webview changes, run `npm run build:webview` and reload the webview.

### Running Tests

```bash
# Extension tests (if any)
npm test

# Webview tests (if any)
cd webview && npm test
```

### Packaging for Distribution

```bash
cd studio
npx @vscode/vsce package --no-dependencies
# Creates: eventflux-studio-x.x.x.vsix
```

### Installing the VSIX Locally

```bash
code --install-extension eventflux-studio-0.1.0.vsix
```

Or in VS Code: Extensions → "..." menu → "Install from VSIX..."

## User Stories

### US-1: Create Simple Filter Query
> As a developer, I want to drag a Stream element onto the canvas, configure its schema, add a Filter element, connect them, and see the generated SQL.

**Status**: COMPLETE

### US-2: Build Aggregation Pipeline
> As a data analyst, I want to visually create a query that calculates average price per symbol over a 5-minute tumbling window.

**Status**: COMPLETE

### US-3: Configure External Connectors
> As a developer, I want to configure a RabbitMQ source with all required parameters shown dynamically based on the connector schema.

**Status**: COMPLETE (schema-based forms)

### US-4: Design Pattern Detection
> As a fraud analyst, I want to visually build a pattern that detects 3 consecutive failed login attempts within 1 minute.

**Status**: PARTIAL (basic pattern support)

### US-5: Test Query with Simulation
> As a developer, I want to test my query with mock data before deploying to production.

**Status**: PENDING

## References

- [EventFlux SQL Grammar](../../vendor/datafusion-sqlparser-rs/)
- [Query API Structures](../../src/query_api/)
- [SQL Compiler](../../src/sql_compiler/)
- [Schema Generator](../../src/bin/generate_schema.rs)

## Related Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) - Detailed technical architecture
