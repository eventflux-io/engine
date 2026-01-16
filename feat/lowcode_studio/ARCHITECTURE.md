# EventFlux Low-Code Studio - Technical Architecture

## VS Code Extension Architecture

### Extension Structure

```
eventflux-studio/
├── package.json                 # Extension manifest
├── src/
│   ├── extension.ts             # Extension entry point
│   ├── commands/                # VS Code commands
│   │   ├── openStudio.ts
│   │   ├── saveApplication.ts
│   │   └── runSimulation.ts
│   ├── providers/               # VS Code providers
│   │   ├── studioEditorProvider.ts
│   │   └── eventfluxLanguageProvider.ts
│   ├── services/                # Backend services
│   │   ├── engineClient.ts      # EventFlux engine connection
│   │   ├── simulationEngine.ts  # Local simulation
│   │   ├── queryGenerator.ts    # Visual to SQL
│   │   └── queryParser.ts       # SQL to Visual
│   └── utils/
├── webview/                     # React application
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/
│   │   │   ├── Canvas/
│   │   │   ├── Palette/
│   │   │   ├── Properties/
│   │   │   ├── Toolbar/
│   │   │   └── QueryEditor/
│   │   ├── elements/            # Element type definitions
│   │   ├── hooks/
│   │   ├── stores/              # Zustand stores
│   │   └── utils/
│   └── index.html
└── test/
```

### Extension Activation

```typescript
// extension.ts
import * as vscode from 'vscode';
import { StudioEditorProvider } from './providers/studioEditorProvider';

export function activate(context: vscode.ExtensionContext) {
  // Register custom editor for .eventflux.studio files
  context.subscriptions.push(
    StudioEditorProvider.register(context)
  );

  // Register command to open studio for .eventflux files
  context.subscriptions.push(
    vscode.commands.registerCommand('eventflux.openStudio', async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && editor.document.fileName.endsWith('.eventflux')) {
        await openStudioForFile(editor.document.uri);
      }
    })
  );

  // Register simulation command
  context.subscriptions.push(
    vscode.commands.registerCommand('eventflux.runSimulation', runSimulation)
  );
}
```

### Custom Editor Provider

```typescript
// studioEditorProvider.ts
export class StudioEditorProvider implements vscode.CustomTextEditorProvider {
  public static register(context: vscode.ExtensionContext): vscode.Disposable {
    const provider = new StudioEditorProvider(context);
    return vscode.window.registerCustomEditorProvider(
      'eventflux.studioEditor',
      provider,
      {
        webviewOptions: {
          retainContextWhenHidden: true,
        },
        supportsMultipleEditorsPerDocument: false,
      }
    );
  }

  async resolveCustomTextEditor(
    document: vscode.TextDocument,
    webviewPanel: vscode.WebviewPanel,
    _token: vscode.CancellationToken
  ): Promise<void> {
    // Set up webview
    webviewPanel.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.context.extensionUri, 'webview', 'dist')
      ]
    };

    // Load React app
    webviewPanel.webview.html = this.getHtmlForWebview(webviewPanel.webview);

    // Handle messages from webview
    webviewPanel.webview.onDidReceiveMessage(
      message => this.handleMessage(document, webviewPanel, message)
    );

    // Initialize with document content
    this.updateWebview(webviewPanel.webview, document);
  }

  private handleMessage(
    document: vscode.TextDocument,
    panel: vscode.WebviewPanel,
    message: any
  ) {
    switch (message.type) {
      case 'update':
        this.updateDocument(document, message.content);
        break;
      case 'generateSQL':
        this.generateAndShowSQL(message.application);
        break;
      case 'runSimulation':
        this.runSimulation(message.application, message.testData);
        break;
      case 'deployToEngine':
        this.deployToEngine(message.application);
        break;
    }
  }
}
```

## Webview React Application

### Component Hierarchy

```
App
├── AppHeader
│   ├── Logo
│   ├── AppName
│   └── ActionButtons (Save, Run, Simulate)
├── MainLayout
│   ├── LeftSidebar
│   │   └── ElementPalette
│   │       ├── PaletteCategory (Sources)
│   │       │   ├── PaletteItem (Stream)
│   │       │   ├── PaletteItem (Table)
│   │       │   └── PaletteItem (Trigger)
│   │       ├── PaletteCategory (Processing)
│   │       │   ├── PaletteItem (Window)
│   │       │   ├── PaletteItem (Filter)
│   │       │   ├── PaletteItem (Projection)
│   │       │   ├── PaletteItem (Aggregation)
│   │       │   ├── PaletteItem (GroupBy)
│   │       │   ├── PaletteItem (Join)
│   │       │   ├── PaletteItem (Pattern)
│   │       │   └── PaletteItem (Partition)
│   │       └── PaletteCategory (Sinks)
│   │           └── PaletteItem (Output)
│   ├── CanvasArea
│   │   └── ReactFlowCanvas
│   │       ├── StreamNode
│   │       ├── WindowNode
│   │       ├── FilterNode
│   │       ├── JoinNode
│   │       ├── PatternNode
│   │       ├── OutputNode
│   │       └── ConnectionEdge
│   └── RightSidebar
│       └── PropertiesPanel
│           ├── ElementHeader
│           ├── PropertySections[]
│           │   ├── BasicProperties
│           │   ├── SchemaEditor
│           │   ├── ExpressionBuilder
│           │   └── WindowConfig
│           └── ValidationMessages
├── BottomPanel (collapsible)
│   ├── TabBar (SQL | Simulation | Output)
│   ├── SQLEditor (Monaco)
│   ├── SimulationPanel
│   │   ├── TestDataInput
│   │   ├── EventTimeline
│   │   └── StateInspector
│   └── OutputPanel
│       └── EventTable
└── StatusBar
    ├── ConnectionStatus
    ├── ValidationStatus
    └── LastSaved
```

### State Management (Zustand)

```typescript
// stores/applicationStore.ts
interface ApplicationState {
  // Application data
  application: VisualApplication;

  // Selection state
  selectedElementIds: string[];
  selectedConnectionId: string | null;

  // UI state
  viewMode: 'visual' | 'sql' | 'split';
  zoom: number;
  pan: { x: number; y: number };

  // Sync state
  generatedSQL: string;
  isDirty: boolean;

  // Actions
  addElement: (element: VisualElement) => void;
  removeElement: (id: string) => void;
  updateElement: (id: string, changes: Partial<VisualElement>) => void;
  moveElement: (id: string, position: Position) => void;

  addConnection: (connection: Connection) => void;
  removeConnection: (id: string) => void;

  selectElement: (id: string, multi?: boolean) => void;
  clearSelection: () => void;

  setViewMode: (mode: 'visual' | 'sql' | 'split') => void;
  setZoom: (zoom: number) => void;
  setPan: (pan: Position) => void;

  regenerateSQL: () => void;
  parseSQL: (sql: string) => void;
}

export const useApplicationStore = create<ApplicationState>()(
  subscribeWithSelector(
    persist(
      (set, get) => ({
        application: createEmptyApplication(),
        selectedElementIds: [],
        selectedConnectionId: null,
        viewMode: 'split',
        zoom: 1,
        pan: { x: 0, y: 0 },
        generatedSQL: '',
        isDirty: false,

        addElement: (element) => {
          set((state) => ({
            application: {
              ...state.application,
              elements: [...state.application.elements, element],
            },
            isDirty: true,
          }));
          get().regenerateSQL();
        },

        // ... other actions
      }),
      { name: 'eventflux-studio' }
    )
  )
);
```

### Element Node Components

```typescript
// components/Canvas/nodes/StreamNode.tsx
import { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { StreamSourceProperties } from '../../../types';

export const StreamNode = memo(({ id, data, selected }: NodeProps<StreamSourceProperties>) => {
  return (
    <div className={`node stream-node ${selected ? 'selected' : ''}`}>
      <div className="node-header">
        <DatabaseIcon className="node-icon" />
        <span className="node-title">{data.streamName || 'Stream'}</span>
      </div>

      <div className="node-body">
        <div className="schema-preview">
          {data.attributes?.slice(0, 3).map((attr, i) => (
            <div key={i} className="attribute">
              <span className="attr-name">{attr.name}</span>
              <span className="attr-type">{attr.type}</span>
            </div>
          ))}
          {data.attributes?.length > 3 && (
            <div className="more">+{data.attributes.length - 3} more</div>
          )}
        </div>
      </div>

      {/* Output port */}
      <Handle
        type="source"
        position={Position.Right}
        id="output"
        className="port output-port"
      />
    </div>
  );
});

// components/Canvas/nodes/WindowNode.tsx
export const WindowNode = memo(({ id, data, selected }: NodeProps<WindowProperties>) => {
  const windowLabel = getWindowLabel(data.windowType, data.parameters);

  return (
    <div className={`node window-node ${selected ? 'selected' : ''}`}>
      {/* Input port */}
      <Handle
        type="target"
        position={Position.Left}
        id="input"
        className="port input-port"
      />

      <div className="node-header">
        <ClockIcon className="node-icon" />
        <span className="node-title">Window</span>
      </div>

      <div className="node-body">
        <div className="window-type">{data.windowType}</div>
        <div className="window-params">{windowLabel}</div>
      </div>

      {/* Output port */}
      <Handle
        type="source"
        position={Position.Right}
        id="output"
        className="port output-port"
      />
    </div>
  );
});

// components/Canvas/nodes/PatternNode.tsx
export const PatternNode = memo(({ id, data, selected }: NodeProps<PatternProperties>) => {
  return (
    <div className={`node pattern-node ${selected ? 'selected' : ''}`}>
      {/* Multiple input ports for pattern streams */}
      <Handle
        type="target"
        position={Position.Left}
        id="input-1"
        style={{ top: '30%' }}
        className="port input-port"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="input-2"
        style={{ top: '70%' }}
        className="port input-port"
      />

      <div className="node-header">
        <RouteIcon className="node-icon" />
        <span className="node-title">{data.mode.toUpperCase()}</span>
      </div>

      <div className="node-body">
        <div className="pattern-preview">
          {renderPatternPreview(data.patternExpression)}
        </div>
        {data.withinConstraint && (
          <div className="within-constraint">
            WITHIN {data.withinConstraint.value} {data.withinConstraint.unit}
          </div>
        )}
      </div>

      <Handle
        type="source"
        position={Position.Right}
        id="output"
        className="port output-port"
      />
    </div>
  );
});

// components/Canvas/nodes/JoinNode.tsx
export const JoinNode = memo(({ id, data, selected }: NodeProps<JoinProperties>) => {
  return (
    <div className={`node join-node ${selected ? 'selected' : ''}`}>
      {/* Left input */}
      <Handle
        type="target"
        position={Position.Left}
        id="left"
        style={{ top: '30%' }}
        className="port input-port"
      />
      {/* Right input */}
      <Handle
        type="target"
        position={Position.Left}
        id="right"
        style={{ top: '70%' }}
        className="port input-port"
      />

      <div className="node-header">
        <MergeIcon className="node-icon" />
        <span className="node-title">{data.joinType.replace('_', ' ').toUpperCase()}</span>
      </div>

      <div className="node-body">
        <div className="join-condition">
          ON {renderExpression(data.onCondition)}
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        id="output"
        className="port output-port"
      />
    </div>
  );
});
```

### Properties Panel

```typescript
// components/Properties/PropertiesPanel.tsx
export const PropertiesPanel: React.FC = () => {
  const selectedElementIds = useApplicationStore(state => state.selectedElementIds);
  const application = useApplicationStore(state => state.application);
  const updateElement = useApplicationStore(state => state.updateElement);

  if (selectedElementIds.length === 0) {
    return <EmptyPropertiesPanel />;
  }

  if (selectedElementIds.length > 1) {
    return <MultiSelectPropertiesPanel count={selectedElementIds.length} />;
  }

  const element = application.elements.find(e => e.id === selectedElementIds[0]);
  if (!element) return null;

  const PropertyEditor = getPropertyEditor(element.type);

  return (
    <div className="properties-panel">
      <div className="panel-header">
        <ElementIcon type={element.type} />
        <span className="element-type">{formatElementType(element.type)}</span>
      </div>

      <div className="panel-content">
        <PropertyEditor
          element={element}
          onChange={(changes) => updateElement(element.id, changes)}
        />
      </div>

      <ValidationMessages elementId={element.id} />
    </div>
  );
};

// Property editors for each element type
function getPropertyEditor(type: ElementType): React.FC<PropertyEditorProps> {
  switch (type) {
    case 'stream_source':
      return StreamPropertyEditor;
    case 'window':
      return WindowPropertyEditor;
    case 'filter':
      return FilterPropertyEditor;
    case 'join':
      return JoinPropertyEditor;
    case 'pattern':
      return PatternPropertyEditor;
    // ... etc
  }
}
```

### Window Property Editor

```typescript
// components/Properties/editors/WindowPropertyEditor.tsx
export const WindowPropertyEditor: React.FC<PropertyEditorProps> = ({ element, onChange }) => {
  const props = element.properties as WindowProperties;

  return (
    <div className="property-editor window-editor">
      <PropertySection title="Window Type">
        <Select
          value={props.windowType}
          onChange={(value) => onChange({
            properties: { ...props, windowType: value, parameters: getDefaultParams(value) }
          })}
          options={[
            { value: 'tumbling', label: 'Tumbling (Fixed non-overlapping)' },
            { value: 'sliding', label: 'Sliding (Overlapping)' },
            { value: 'length', label: 'Length (Last N events)' },
            { value: 'lengthBatch', label: 'Length Batch (Batch of N)' },
            { value: 'time', label: 'Time (Rolling duration)' },
            { value: 'timeBatch', label: 'Time Batch (Periodic)' },
            { value: 'session', label: 'Session (Gap-based)' },
            { value: 'externalTime', label: 'External Time (Event timestamp)' },
            { value: 'externalTimeBatch', label: 'External Time Batch' },
            { value: 'sort', label: 'Sort (Top N sorted)' },
          ]}
        />
      </PropertySection>

      {/* Dynamic parameters based on window type */}
      {needsDuration(props.windowType) && (
        <PropertySection title="Duration">
          <TimeIntervalInput
            value={props.parameters.duration}
            onChange={(duration) => onChange({
              properties: { ...props, parameters: { ...props.parameters, duration }}
            })}
          />
        </PropertySection>
      )}

      {props.windowType === 'sliding' && (
        <PropertySection title="Slide Interval">
          <TimeIntervalInput
            value={props.parameters.slideInterval}
            onChange={(slideInterval) => onChange({
              properties: { ...props, parameters: { ...props.parameters, slideInterval }}
            })}
          />
        </PropertySection>
      )}

      {needsCount(props.windowType) && (
        <PropertySection title="Count">
          <NumberInput
            value={props.parameters.count}
            min={1}
            onChange={(count) => onChange({
              properties: { ...props, parameters: { ...props.parameters, count }}
            })}
          />
        </PropertySection>
      )}

      {needsTimestampAttribute(props.windowType) && (
        <PropertySection title="Timestamp Attribute">
          <AttributeSelect
            value={props.parameters.timestampAttribute}
            availableAttributes={getUpstreamAttributes(element.id)}
            onChange={(timestampAttribute) => onChange({
              properties: { ...props, parameters: { ...props.parameters, timestampAttribute }}
            })}
          />
        </PropertySection>
      )}
    </div>
  );
};
```

### Pattern Builder

```typescript
// components/Properties/editors/PatternPropertyEditor.tsx
export const PatternPropertyEditor: React.FC<PropertyEditorProps> = ({ element, onChange }) => {
  const props = element.properties as PatternProperties;
  const [showAdvancedBuilder, setShowAdvancedBuilder] = useState(false);

  return (
    <div className="property-editor pattern-editor">
      <PropertySection title="Mode">
        <RadioGroup
          value={props.mode}
          onChange={(mode) => onChange({ properties: { ...props, mode }})}
          options={[
            { value: 'pattern', label: 'PATTERN', description: 'Relaxed matching (allows gaps)' },
            { value: 'sequence', label: 'SEQUENCE', description: 'Strict consecutive' },
          ]}
        />
      </PropertySection>

      <PropertySection title="Pattern Expression">
        {showAdvancedBuilder ? (
          <PatternVisualBuilder
            expression={props.patternExpression}
            onChange={(patternExpression) => onChange({
              properties: { ...props, patternExpression }
            })}
          />
        ) : (
          <PatternTextEditor
            expression={props.patternExpression}
            onChange={(patternExpression) => onChange({
              properties: { ...props, patternExpression }
            })}
          />
        )}
        <Button
          variant="link"
          onClick={() => setShowAdvancedBuilder(!showAdvancedBuilder)}
        >
          {showAdvancedBuilder ? 'Switch to Text' : 'Switch to Visual Builder'}
        </Button>
      </PropertySection>

      <PropertySection title="WITHIN Constraint">
        <Toggle
          checked={!!props.withinConstraint}
          onChange={(enabled) => onChange({
            properties: {
              ...props,
              withinConstraint: enabled ? { type: 'time', value: 60, unit: 'SECONDS' } : undefined
            }
          })}
          label="Enable WITHIN"
        />

        {props.withinConstraint && (
          <div className="within-config">
            <RadioGroup
              value={props.withinConstraint.type}
              onChange={(type) => onChange({
                properties: {
                  ...props,
                  withinConstraint: { ...props.withinConstraint!, type }
                }
              })}
              options={[
                { value: 'time', label: 'Time' },
                { value: 'event_count', label: 'Event Count' },
              ]}
            />

            {props.withinConstraint.type === 'time' ? (
              <TimeIntervalInput
                value={{ value: props.withinConstraint.value, unit: props.withinConstraint.unit! }}
                onChange={(interval) => onChange({
                  properties: {
                    ...props,
                    withinConstraint: {
                      type: 'time',
                      value: interval.value,
                      unit: interval.unit
                    }
                  }
                })}
              />
            ) : (
              <NumberInput
                value={props.withinConstraint.value}
                min={1}
                suffix="events"
                onChange={(value) => onChange({
                  properties: {
                    ...props,
                    withinConstraint: { type: 'event_count', value }
                  }
                })}
              />
            )}
          </div>
        )}
      </PropertySection>
    </div>
  );
};

// Visual pattern builder component
const PatternVisualBuilder: React.FC<{
  expression: PatternExpressionModel;
  onChange: (expr: PatternExpressionModel) => void;
}> = ({ expression, onChange }) => {
  return (
    <div className="pattern-visual-builder">
      <div className="pattern-elements">
        {/* Render pattern tree visually */}
        <PatternNode
          node={expression}
          onUpdate={onChange}
          depth={0}
        />
      </div>

      <div className="pattern-toolbar">
        <Button onClick={() => addOperator('->')}>Add Followed-By (->)</Button>
        <Button onClick={() => addOperator('and')}>Add AND</Button>
        <Button onClick={() => addOperator('or')}>Add OR</Button>
        <Button onClick={() => wrapWithEvery()}>Wrap with EVERY</Button>
        <Button onClick={() => addCountQuantifier()}>Add Count {'{n}'}</Button>
      </div>

      <div className="pattern-preview">
        <code>{patternToString(expression)}</code>
      </div>
    </div>
  );
};
```

## Query Generation

```typescript
// services/queryGenerator.ts
export class QueryGenerator {
  generate(app: VisualApplication): string {
    const statements: string[] = [];

    // 1. Generate DDL (CREATE STREAM/TABLE/TRIGGER)
    for (const element of app.elements) {
      if (element.type === 'stream_source') {
        statements.push(this.generateStreamDDL(element));
      } else if (element.type === 'table_source') {
        statements.push(this.generateTableDDL(element));
      } else if (element.type === 'trigger_source') {
        statements.push(this.generateTriggerDDL(element));
      }
    }

    // 2. Find query paths (source -> ... -> output)
    const queryPaths = this.findQueryPaths(app);

    // 3. Generate each query
    for (const path of queryPaths) {
      statements.push(this.generateQuery(path, app));
    }

    return statements.join('\n\n');
  }

  private generateQuery(path: VisualElement[], app: VisualApplication): string {
    const parts: QueryParts = {
      insertInto: '',
      select: [],
      from: '',
      window: '',
      where: '',
      groupBy: [],
      having: '',
      orderBy: [],
      limit: undefined,
    };

    // Process elements in order
    for (const element of path) {
      this.processElement(element, parts, app);
    }

    return this.assembleQuery(parts);
  }

  private processElement(element: VisualElement, parts: QueryParts, app: VisualApplication): void {
    switch (element.type) {
      case 'stream_source':
        parts.from = (element.properties as StreamSourceProperties).streamName;
        break;

      case 'window':
        parts.window = this.generateWindowClause(element.properties as WindowProperties);
        break;

      case 'filter':
        parts.where = this.expressionToSQL((element.properties as FilterProperties).condition);
        break;

      case 'projection':
        const projProps = element.properties as ProjectionProperties;
        parts.select = projProps.selectList.map(attr => {
          const expr = this.expressionToSQL(attr.expression);
          return attr.alias ? `${expr} AS ${attr.alias}` : expr;
        });
        break;

      case 'aggregation':
        const aggProps = element.properties as AggregationProperties;
        for (const agg of aggProps.aggregations) {
          const expr = `${agg.type}(${this.expressionToSQL(agg.expression)})`;
          parts.select.push(`${expr} AS ${agg.alias}`);
        }
        break;

      case 'group_by':
        const gbProps = element.properties as GroupByProperties;
        parts.groupBy = gbProps.groupByAttributes;
        if (gbProps.havingCondition) {
          parts.having = this.expressionToSQL(gbProps.havingCondition);
        }
        break;

      case 'join':
        parts.from = this.generateJoinClause(element, parts.from, app);
        break;

      case 'pattern':
        parts.from = this.generatePatternClause(element.properties as PatternProperties);
        break;

      case 'output_stream':
        parts.insertInto = (element.properties as OutputProperties).targetStreamName;
        break;
    }
  }

  private generateWindowClause(props: WindowProperties): string {
    const { windowType, parameters } = props;

    switch (windowType) {
      case 'tumbling':
        return `WINDOW('tumbling', ${this.timeToSQL(parameters.duration!)})`;
      case 'sliding':
        return `WINDOW('sliding', ${this.timeToSQL(parameters.duration!)}, ${this.timeToSQL(parameters.slideInterval!)})`;
      case 'length':
        return `WINDOW('length', ${parameters.count})`;
      case 'lengthBatch':
        return `WINDOW('lengthBatch', ${parameters.count})`;
      case 'time':
        return `WINDOW('time', ${this.timeToSQL(parameters.duration!)})`;
      case 'timeBatch':
        return `WINDOW('timeBatch', ${this.timeToSQL(parameters.duration!)})`;
      case 'session':
        return `WINDOW('session', ${this.timeToSQL(parameters.gapDuration!)})`;
      case 'externalTime':
        return `WINDOW('externalTime', ${parameters.timestampAttribute}, ${this.timeToSQL(parameters.duration!)})`;
      case 'externalTimeBatch':
        return `WINDOW('externalTimeBatch', ${parameters.timestampAttribute}, ${this.timeToSQL(parameters.duration!)})`;
      case 'sort':
        return `WINDOW('sort', ${parameters.count}, ${parameters.sortAttribute})`;
      default:
        return '';
    }
  }

  private generatePatternClause(props: PatternProperties): string {
    const mode = props.mode.toUpperCase();
    const pattern = this.patternToSQL(props.patternExpression);
    let clause = `${mode} (${pattern})`;

    if (props.withinConstraint) {
      const { type, value, unit } = props.withinConstraint;
      if (type === 'time') {
        clause += ` WITHIN ${value} ${unit}`;
      } else {
        clause += ` WITHIN ${value} EVENTS`;
      }
    }

    return clause;
  }

  private patternToSQL(expr: PatternExpressionModel): string {
    switch (expr.type) {
      case 'stream':
        let s = expr.streamAlias ? `${expr.streamAlias}=${expr.streamName}` : expr.streamName!;
        if (expr.filter) {
          s += `[${this.expressionToSQL(expr.filter)}]`;
        }
        if (expr.countMin !== undefined) {
          if (expr.countMin === expr.countMax) {
            s += `{${expr.countMin}}`;
          } else {
            s += `{${expr.countMin},${expr.countMax}}`;
          }
        }
        return s;

      case 'next':
        return `${this.patternToSQL(expr.left!)} -> ${this.patternToSQL(expr.right!)}`;

      case 'logical':
        return `(${this.patternToSQL(expr.left!)} ${expr.operator!.toUpperCase()} ${this.patternToSQL(expr.right!)})`;

      case 'every':
        return `EVERY(${this.patternToSQL(expr.operand!)})`;

      case 'absent':
        return `NOT ${expr.streamName} FOR ${this.timeToSQL(expr.duration!)}`;

      default:
        return '';
    }
  }

  private assembleQuery(parts: QueryParts): string {
    let sql = '';

    // INSERT INTO (at beginning for EventFlux style)
    if (parts.insertInto) {
      sql += `INSERT INTO ${parts.insertInto}\n`;
    }

    // SELECT
    sql += 'SELECT ';
    if (parts.select.length > 0) {
      sql += parts.select.join(', ');
    } else {
      sql += '*';
    }

    // FROM
    sql += `\nFROM ${parts.from}`;
    if (parts.window) {
      sql += ` ${parts.window}`;
    }

    // WHERE
    if (parts.where) {
      sql += `\nWHERE ${parts.where}`;
    }

    // GROUP BY
    if (parts.groupBy.length > 0) {
      sql += `\nGROUP BY ${parts.groupBy.join(', ')}`;
    }

    // HAVING
    if (parts.having) {
      sql += `\nHAVING ${parts.having}`;
    }

    // ORDER BY
    if (parts.orderBy.length > 0) {
      sql += `\nORDER BY ${parts.orderBy.join(', ')}`;
    }

    // LIMIT
    if (parts.limit !== undefined) {
      sql += `\nLIMIT ${parts.limit}`;
    }

    return sql + ';';
  }
}
```

## Simulation Engine

```typescript
// services/simulationEngine.ts
export class SimulationEngine {
  private elements: Map<string, SimulationElement>;
  private connections: Map<string, string[]>;
  private eventQueue: PriorityQueue<SimEvent>;
  private state: Map<string, any>;

  async simulate(
    app: VisualApplication,
    testData: TestDataSet[],
    options: SimulationOptions
  ): Promise<SimulationResult> {
    // Initialize simulation state
    this.initialize(app);

    // Load test data into event queue
    for (const data of testData) {
      for (const event of data.events) {
        this.eventQueue.push({
          timestamp: event.timestamp,
          streamName: data.streamName,
          data: event.data,
        });
      }
    }

    // Process events
    const results: SimulationStep[] = [];
    let stepCount = 0;

    while (!this.eventQueue.isEmpty() && stepCount < options.maxSteps) {
      const event = this.eventQueue.pop();
      const step = await this.processEvent(event);
      results.push(step);
      stepCount++;

      // Emit progress
      options.onProgress?.(stepCount, this.eventQueue.size());
    }

    return {
      steps: results,
      finalState: this.getState(),
      statistics: this.calculateStatistics(results),
    };
  }

  private async processEvent(event: SimEvent): Promise<SimulationStep> {
    const step: SimulationStep = {
      timestamp: event.timestamp,
      inputEvent: event,
      elementStates: new Map(),
      outputEvents: [],
    };

    // Find source element for this stream
    const sourceElement = this.findSourceElement(event.streamName);
    if (!sourceElement) return step;

    // Propagate through connected elements
    await this.propagate(sourceElement, event, step);

    return step;
  }

  private async propagate(
    element: SimulationElement,
    event: SimEvent,
    step: SimulationStep
  ): Promise<void> {
    // Process event through this element
    const result = await element.process(event, this.state.get(element.id));

    // Save element state
    step.elementStates.set(element.id, {
      before: this.state.get(element.id),
      after: result.newState,
    });
    this.state.set(element.id, result.newState);

    // If element produces output, propagate to connected elements
    if (result.outputEvents.length > 0) {
      const downstream = this.connections.get(element.id) || [];
      for (const nextId of downstream) {
        const nextElement = this.elements.get(nextId);
        if (nextElement) {
          for (const outEvent of result.outputEvents) {
            await this.propagate(nextElement, outEvent, step);
          }
        }
      }
    }

    // If this is an output element, add to step outputs
    if (element.type === 'output_stream') {
      step.outputEvents.push(...result.outputEvents);
    }
  }
}

// Simulation element implementations
class WindowSimElement implements SimulationElement {
  type = 'window';

  async process(event: SimEvent, state: WindowState): Promise<ProcessResult> {
    const newState = { ...state };
    const outputEvents: SimEvent[] = [];

    // Add event to window buffer
    newState.buffer.push(event);

    // Apply window logic based on type
    switch (this.config.windowType) {
      case 'length':
        // Keep only last N events
        while (newState.buffer.length > this.config.parameters.count!) {
          newState.buffer.shift();
        }
        outputEvents.push({ ...event, windowState: [...newState.buffer] });
        break;

      case 'lengthBatch':
        // Emit when batch is full
        if (newState.buffer.length >= this.config.parameters.count!) {
          outputEvents.push({
            timestamp: event.timestamp,
            streamName: event.streamName,
            data: { batch: [...newState.buffer] },
          });
          newState.buffer = [];
        }
        break;

      // ... other window types
    }

    return { newState, outputEvents };
  }
}
```

## Communication Protocol

### Webview to Extension Host

```typescript
// Message types from webview to extension
type WebviewMessage =
  | { type: 'ready' }
  | { type: 'save'; content: string }
  | { type: 'generateSQL'; application: VisualApplication }
  | { type: 'parseSQL'; sql: string }
  | { type: 'runSimulation'; application: VisualApplication; testData: TestDataSet[] }
  | { type: 'deployToEngine'; application: VisualApplication }
  | { type: 'sendTestEvent'; streamName: string; data: any }
  | { type: 'stopExecution' }
  | { type: 'getAvailableStreams' }
  | { type: 'getBuiltInFunctions' };

// Message types from extension to webview
type ExtensionMessage =
  | { type: 'load'; content: string }
  | { type: 'sqlGenerated'; sql: string }
  | { type: 'applicationParsed'; application: VisualApplication }
  | { type: 'simulationStep'; step: SimulationStep }
  | { type: 'simulationComplete'; result: SimulationResult }
  | { type: 'executionOutput'; events: OutputEvent[] }
  | { type: 'validationResult'; errors: ValidationError[] }
  | { type: 'catalogUpdate'; streams: StreamDefinition[]; functions: FunctionSignature[] };
```

## File Format

### .eventflux.studio (Visual Project)

```json
{
  "$schema": "https://eventflux.io/schemas/studio/v1.json",
  "version": "1.0",
  "name": "Stock Price Analysis",
  "description": "Analyze stock prices with tumbling windows",
  "application": {
    "elements": [
      {
        "id": "stream-1",
        "type": "stream_source",
        "position": { "x": 100, "y": 200 },
        "size": { "width": 160, "height": 100 },
        "properties": {
          "streamName": "StockTicks",
          "attributes": [
            { "name": "symbol", "type": "STRING" },
            { "name": "price", "type": "DOUBLE" },
            { "name": "volume", "type": "LONG" }
          ]
        }
      },
      {
        "id": "window-1",
        "type": "window",
        "position": { "x": 350, "y": 200 },
        "size": { "width": 140, "height": 80 },
        "properties": {
          "windowType": "tumbling",
          "parameters": {
            "duration": { "value": 5, "unit": "MINUTES" }
          }
        }
      }
    ],
    "connections": [
      {
        "id": "conn-1",
        "sourceElementId": "stream-1",
        "sourcePortId": "output",
        "targetElementId": "window-1",
        "targetPortId": "input"
      }
    ]
  },
  "layout": {
    "zoom": 1.0,
    "pan": { "x": 0, "y": 0 },
    "gridSize": 20,
    "snapToGrid": true
  },
  "simulation": {
    "testDataSets": [
      {
        "name": "Sample Stock Data",
        "streamName": "StockTicks",
        "events": [
          { "timestamp": 1000, "data": { "symbol": "AAPL", "price": 150.0, "volume": 1000 } }
        ]
      }
    ]
  }
}
```

## Schema Generation System

The Studio uses build-time schema generation to provide dynamic configuration forms without requiring the EventFlux engine to run.

### Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                         BUILD TIME                                    │
│  ┌─────────────────────┐                                             │
│  │  EventFluxContext   │  Registers all factories:                   │
│  │  (Rust)             │  - SourceFactory (rabbitmq, websocket, etc) │
│  │                     │  - SinkFactory (log, rabbitmq, etc)         │
│  │                     │  - WindowProcessorFactory (10 types)        │
│  │                     │  - ScalarFunctionFactory (45 functions)     │
│  │                     │  - AggregatorFactory (8 aggregators)        │
│  └─────────┬───────────┘                                             │
│            │                                                          │
│            ▼                                                          │
│  ┌─────────────────────┐      ┌──────────────────────────────────┐  │
│  │  generate_schema.rs │ ──►  │  eventflux-schema.json           │  │
│  │  (Binary)           │      │  - sources: { rabbitmq, ... }    │  │
│  │                     │      │  - sinks: { log, ... }           │  │
│  │                     │      │  - windows: { time, length, ... }│  │
│  │                     │      │  - functions: { abs, concat, ...}│  │
│  └─────────────────────┘      └──────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│                         RUNTIME                                       │
│  ┌──────────────────────────────────┐                                │
│  │  schemas/index.ts                │                                │
│  │  - getSourceTypes() ────────────────► Source dropdown             │
│  │  - getSinkTypes() ──────────────────► Sink dropdown               │
│  │  - getSourceSchema(type) ───────────► Dynamic parameter forms     │
│  │  - getConnectorParameters(schema) ──► Required/optional fields    │
│  │  - formatParameterName(key) ────────► "rabbitmq.host" → "Host"    │
│  └──────────────────────────────────┘                                │
│                                                                       │
│  Future integrations:                                                 │
│  - getWindowTypes() ───────────────────► Window type picker          │
│  - getAggregatorTypes() ───────────────► Aggregation function picker │
│  - getFunctionNames() ─────────────────► Expression autocomplete     │
│  - getMapperTypes() ───────────────────► Format picker (json, csv)   │
│  - getTableExtensions() ───────────────► Table extension dropdown    │
└──────────────────────────────────────────────────────────────────────┘
```

### Schema Generator Binary

Located at `src/bin/generate_schema.rs`:

```rust
fn main() {
    let ctx = EventFluxContext::new();  // All factories auto-registered
    let schema = generate_schema(&ctx);
    let json = serde_json::to_string_pretty(&schema)?;
    fs::write("studio/webview/src/schemas/eventflux-schema.json", json)?;
}
```

### Generated Schema Structure

```json
{
  "version": "0.1.0",
  "generated": "2024-01-10T...",
  "sources": {
    "rabbitmq": {
      "name": "rabbitmq",
      "supportedFormats": ["json", "csv", "bytes"],
      "requiredParameters": ["rabbitmq.host", "rabbitmq.queue"],
      "optionalParameters": ["rabbitmq.port", "rabbitmq.vhost", ...]
    }
  },
  "sinks": { ... },
  "mappers": { ... },
  "windows": { ... },
  "aggregators": { ... },
  "functions": { ... },
  "collectionAggregators": { ... },
  "tables": { ... }
}
```

### TypeScript Schema Module

Located at `studio/webview/src/schemas/index.ts`:

```typescript
// Currently used (Phase 1)
export function getSourceTypes(): string[]
export function getSinkTypes(): string[]
export function getSourceSchema(type): ConnectorSchema
export function getSinkSchema(type): ConnectorSchema
export function getConnectorParameters(schema): Param[]
export function formatParameterName(key): string

// Reserved for future use (Phase 3)
export function getMapperTypes(): string[]
export function getWindowTypes(): string[]
export function getAggregatorTypes(): string[]
export function getFunctionNames(): string[]
export function getTableExtensions(): string[]
export function getMapperSchema(type): MapperSchema
```

#### Currently Used Functions (Phase 1)

| Function | Purpose | Used In |
|----------|---------|---------|
| `getSourceTypes()` | Returns list of available source connectors | Source element type dropdown |
| `getSinkTypes()` | Returns list of available sink connectors | Sink element type dropdown |
| `getSourceSchema(type)` | Gets metadata for a source connector | Source property editor |
| `getSinkSchema(type)` | Gets metadata for a sink connector | Sink property editor |
| `getConnectorParameters(schema)` | Extracts required + optional params | Dynamic form field generation |
| `formatParameterName(key)` | Formats param key for display | Form labels ("rabbitmq.host" → "Host") |

#### Reserved Functions - Future Integration (Phase 3)

| Function | Future Purpose | Integration Point |
|----------|----------------|-------------------|
| `getMapperTypes()` | Format picker dropdown (json, csv, bytes) | Source/Sink config panel - allows selecting data format for connectors that support multiple formats |
| `getWindowTypes()` | Window type dropdown with descriptions | Window element editor - replaces hardcoded window types with schema-driven list including descriptions |
| `getAggregatorTypes()` | Aggregation function picker | Aggregation element - provides dropdown of available aggregators (SUM, AVG, COUNT, MIN, MAX, etc.) with arity info |
| `getFunctionNames()` | Expression builder autocomplete | Filter/Projection expression input - autocomplete suggestions when typing function names (abs, concat, upper, etc.) |
| `getTableExtensions()` | Table extension dropdown | Table element editor - allows selecting table backend (inMemory, cache, jdbc) |
| `getMapperSchema(type)` | Mapper parameter configuration | Source/Sink config - dynamic form for mapper-specific params (e.g., csv.delimiter, json.date-format) |

## CI/CD Pipeline

### GitHub Actions Workflow

Located at `.github/workflows/studio.yml`:

```yaml
jobs:
  generate-schema:
    # 1. Checkout repo
    # 2. Setup Rust
    # 3. cargo run --bin generate_schema
    # 4. Upload schema as artifact

  build:
    needs: generate-schema
    # 1. Checkout repo
    # 2. Download schema artifact
    # 3. npm ci (extension + webview)
    # 4. npm run build
    # 5. vsce package
    # 6. Upload VSIX artifact

  publish:
    needs: build
    if: release OR manual with publish=true
    # 1. Download VSIX
    # 2. vsce publish
    # 3. Attach to GitHub release
```

### Why No Cleanup Needed

The workflow does NOT need `rm -f *.vsix` before packaging because:
- Each GitHub Actions run starts with a **fresh checkout**
- The `studio/` directory contains no pre-existing `.vsix` files
- The `vsce package` command creates the VSIX fresh each time

This is different from local development where repeated packaging accumulates old `.vsix` files.

### Path Filters

The workflow only triggers for changes in:
- `studio/**` - Extension and webview code
- `src/bin/generate_schema.rs` - Schema generator
- `.github/workflows/studio.yml` - Workflow itself

Note: Changes to factory files (e.g., adding new connector parameters) require manual schema regeneration since those paths aren't in the filter.

## Schema Propagation (TODO - Phase 2)

### Overview

The visual editor needs to track attribute schemas as they flow through processing elements. This enables attribute autocomplete, validation, and type checking.

### Schema Flow

```
Stream(symbol, price, volume)
    │
    ▼ upstream: {symbol:STRING, price:DOUBLE, volume:LONG}
Filter[price > 100]
    │
    ▼ upstream: {symbol:STRING, price:DOUBLE, volume:LONG}  (unchanged)
Projection[symbol, price * volume AS value]
    │
    ▼ upstream: {symbol:STRING, value:DOUBLE}  (CHANGED!)
Aggregation[SUM(value) AS total GROUP BY symbol]
    │
    ▼ upstream: {symbol:STRING, total:DOUBLE}  (CHANGED!)
Output
```

### Key Functions Needed

```typescript
// Get attributes available TO an element (from upstream)
getUpstreamSchema(elementId: string): Attribute[]

// Get attributes produced BY an element (for downstream)
getOutputSchema(element: Element): Attribute[]

// Infer type of an expression given available attributes
inferExpressionType(expr: Expression, schema: Attribute[]): AttributeType
```

### Element-Specific Logic

| Element | Output Schema Logic |
|---------|---------------------|
| Stream | Return defined attributes |
| Source | Return defined attributes (from connector) |
| Filter | Pass through upstream unchanged |
| Window | Pass through upstream unchanged |
| Projection | Return selected columns with aliases |
| Aggregation | Return GROUP BY columns + aggregated columns |
| Join | Merge left + right schemas (handle name conflicts) |
| Pattern | Extract from SELECT clause referencing matched events |

### TODOs

- [ ] Implement `getOutputSchema()` for Projection (parse SELECT list)
- [ ] Implement `getOutputSchema()` for Aggregation (GROUP BY + aggregates)
- [ ] Implement schema merging for Join (handle `left.x`, `right.x`)
- [ ] Implement expression type inference
- [ ] Cache computed schemas (invalidate on upstream changes)

## File Format Strategy

### Why Two Separate Extensions?

| Extension | Format | Purpose | Promise to User |
|-----------|--------|---------|-----------------|
| `.eventflux` | SQL | Production queries | "Run me with the EventFlux engine" |
| `.eventflux.studio` | JSON | Visual Studio projects | "Open me in Studio" |

**The file extension is a contract.** Using `.eventflux` for Studio projects would break user expectations:
- Users expect `.eventflux` files to be valid SQL
- Users expect to run `.eventflux` with `cargo run --bin run_eventflux`
- Users expect to edit `.eventflux` with any text editor

### The Data Loss Problem

The visual editor cannot yet represent 100% of EventFlux SQL grammar:

```
┌─────────────────────────────────────────────────────────────────┐
│  User opens complex.eventflux (has WITH clauses, patterns)      │
│                              ▼                                  │
│  Studio can't represent WITH clauses visually                   │
│                              ▼                                  │
│  User makes visual edits, saves                                 │
│                              ▼                                  │
│  WITH clauses SILENTLY LOST ❌                                  │
│                              ▼                                  │
│  Production query is broken, user doesn't know why              │
└─────────────────────────────────────────────────────────────────┘
```

**This is unacceptable.** Data loss must never happen silently.

### Current Approach: Import/Export (Phase 1-2)

```
┌─────────────────┐                         ┌─────────────────────┐
│  .eventflux    │  ──── Import ─────────► │  .eventflux.studio  │
│  (SQL, full    │                          │  (JSON, visual)     │
│   grammar)     │  ◄──── Export ─────────  │                     │
└─────────────────┘         │               └─────────────────────┘
                            │
                            ▼
                  ⚠️ Warning dialog if
                  features will be lost
```

**Import workflow:**
1. File → Import from `.eventflux`
2. SQL parser identifies unsupported features
3. Warning: "This file uses features not yet supported: WITH clause (line 5), PATTERN quantifiers (line 12). Import anyway?"
4. User makes informed decision

**Export workflow:**
1. File → Export to `.eventflux`
2. Generates clean, valid SQL from visual representation
3. No data loss because the visual canvas is the source of truth

### Future: Graceful Degradation (Phase 3+)

Once Studio matures, it will support opening `.eventflux` files directly with preserved "SQL Block" nodes:

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
- Users can edit supported parts visually, unsupported parts preserved
- As Studio adds features, fewer SQL Blocks needed

### Long-term: Sidecar Layout File (Phase 4+)

When Studio covers 90%+ of the grammar:

```
myapp.eventflux        # SQL (source of truth, runnable)
myapp.eventflux.layout # JSON (positions only, auto-generated, optional)
```

- The SQL file is always the source of truth
- Layout file stores only visual positioning
- If layout is missing, auto-layout is used
- Can run `myapp.eventflux` directly without Studio

### Evolution Roadmap

| Phase | Strategy | Rationale |
|-------|----------|-----------|
| Phase 1-2 | Separate `.eventflux.studio` + Import/Export | Prevents data loss, clear expectations |
| Phase 3 | SQL Block nodes for graceful degradation | Lossless round-trip with partial visual editing |
| Phase 4+ | `.eventflux` + `.eventflux.layout` sidecar | Single source of truth when Studio is mature |

### Phase 2 Implementation Tasks

- [ ] Implement "Import from .eventflux" command
- [ ] SQL → Visual parser with unsupported feature detection
- [ ] Warning dialog showing unsupported features
- [ ] Implement "Export to .eventflux" command
- [ ] Visual → SQL generator (already exists, needs refinement)

## Critical Files Reference

| Purpose | EventFlux File |
|---------|---------------|
| Query structure | `src/query_api/execution/query/query.rs` |
| SQL conversion | `src/sql_compiler/converter.rs` |
| Expression types | `src/query_api/expression/expression.rs` |
| Pattern elements | `src/query_api/execution/query/input/state/state_element.rs` |
| Type inference | `src/sql_compiler/type_inference.rs` |
| Window handlers | `src/query_api/execution/query/input/handler/window.rs` |
| Stream definition | `src/query_api/definition/stream_definition.rs` |
| Table definition | `src/query_api/definition/table_definition.rs` |
| Schema generator | `src/bin/generate_schema.rs` |
| EventFlux context | `src/core/config/eventflux_context.rs` |
