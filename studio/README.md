# EventFlux Studio

Visual low-code editor for creating EventFlux streaming applications as a VS Code extension.

## Features

- **Drag & Drop Canvas**: Visually build streaming pipelines with React Flow
- **Element Palette**: Sources (Stream, Table, Trigger), Processing (Window, Filter, Join, Pattern), Sinks (Output)
- **Properties Panel**: Configure elements with schema editors, expression builders
- **SQL Generation**: Auto-generate EventFlux SQL from visual model
- **Bidirectional Sync**: Switch between visual and SQL views
- **Templates**: Pre-built templates for common use cases

## Project Structure

```
studio/
├── src/                          # VS Code extension source
│   ├── extension.ts              # Extension entry point
│   ├── providers/
│   │   └── studioEditorProvider.ts
│   └── services/
├── webview/                      # React application
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/
│   │   │   ├── Canvas/
│   │   │   ├── Palette/
│   │   │   ├── Properties/
│   │   │   └── Toolbar/
│   │   ├── elements/
│   │   ├── stores/
│   │   └── types/
│   └── package.json
├── templates/                    # Pre-built templates
└── package.json                  # Extension manifest
```

## Development

### Prerequisites

- Node.js 18+
- VS Code 1.85+

### Setup

```bash
# Install extension dependencies
cd studio
npm install

# Install webview dependencies
cd webview
npm install
```

### Build

```bash
# Build everything
npm run build

# Or build separately
npm run build:extension    # Build VS Code extension
npm run build:webview      # Build React webview
```

### Development Mode

```bash
# Watch mode for both extension and webview
npm run watch

# Or in separate terminals:
npm run watch:extension
cd webview && npm run dev
```

### Test in VS Code

1. Open the `studio` folder in VS Code
2. Press F5 to launch Extension Development Host
3. In the new window, create a `.eventflux.studio` file or run "EventFlux: New Studio Project"

## Supported Elements

### Sources
- **Stream** - Input event stream with schema
- **Table** - Reference data table for lookups
- **Trigger** - Time-based event generator (START, PERIODIC, CRON)

### Processing
- **Window** - 9 types: length, lengthBatch, time, timeBatch, tumbling, sliding, session, externalTime, externalTimeBatch, sort
- **Filter** - WHERE condition filtering
- **Projection** - SELECT column transformation
- **Aggregation** - COUNT, SUM, AVG, MIN, MAX, etc.
- **Group By** - GROUP BY with HAVING
- **Join** - INNER, LEFT, RIGHT, FULL OUTER joins
- **Pattern** - CEP pattern matching (PATTERN/SEQUENCE)
- **Partition** - Parallel partitioning

### Sinks
- **Output** - INSERT INTO target stream

## File Formats

### .eventflux.studio (Visual Project)
JSON format containing elements, connections, layout, and metadata.

### .eventflux (SQL Query)
Standard EventFlux SQL syntax - can be imported/exported.

## Templates

Pre-built templates in `templates/`:
- `stock-analysis.json` - Stock price analysis with aggregations

## Tech Stack

- **Extension**: TypeScript, esbuild
- **Webview**: React 18, React Flow, Zustand, Tailwind CSS, Vite
- **Icons**: Lucide React

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes and test
4. Submit a pull request

## License

Apache-2.0 - See LICENSE file
