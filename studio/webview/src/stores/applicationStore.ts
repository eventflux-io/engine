import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { Node, Edge, NodeChange, EdgeChange, applyNodeChanges, applyEdgeChanges } from '@xyflow/react';
import type { VisualApplication, VisualElement, Connection, StudioConfig, ElementType, ElementProperties } from '../types';
import { vscode } from '../utils/vscode';
import { parseSQL, validateSQL } from '../utils/sqlParser';
import { getSourceTypes, getSinkTypes, getSourceSchema, getSinkSchema, getWindowTypes } from '../schemas';

// Debounce utility to prevent rapid saves
function debounce<T extends (...args: unknown[]) => void>(fn: T, delay: number): T {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  return ((...args: unknown[]) => {
    if (timeoutId) clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), delay);
  }) as T;
}

// Schema information for upstream attributes
export interface UpstreamAttribute {
  name: string;
  type: string;
  streamId?: string;
  source: string; // element ID that defined this attribute
}

// History snapshot for undo/redo
interface HistorySnapshot {
  nodes: Node[];
  edges: Edge[];
}

interface ApplicationState {
  // Application data
  application: {
    elements: VisualElement[];
    connections: Connection[];
  };
  name: string;
  isDirty: boolean;

  // React Flow state
  nodes: Node[];
  edges: Edge[];

  // Selection state
  selectedElementIds: string[];
  selectedConnectionId: string | null;

  // UI state
  viewMode: 'visual' | 'sql' | 'split';
  generatedSQL: string;

  // Configuration
  config: StudioConfig;

  // Undo/Redo history (past = states to undo to, future = states to redo to)
  past: HistorySnapshot[];
  future: HistorySnapshot[];

  // Actions
  loadApplication: (data: VisualApplication) => void;
  setNodes: (nodes: Node[]) => void;
  setEdges: (edges: Edge[]) => void;
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;

  addElement: (type: ElementType, position: { x: number; y: number }, properties?: Record<string, unknown>) => void;
  removeElement: (id: string) => void;
  updateElement: (id: string, changes: Partial<VisualElement> | { data: Record<string, unknown> }) => void;

  addConnection: (connection: Connection) => void;
  removeConnection: (id: string) => void;

  selectElement: (id: string | null, multi?: boolean) => void;
  clearSelection: () => void;

  setViewMode: (mode: 'visual' | 'sql' | 'split') => void;
  regenerateSQL: () => void;

  // SQL sync
  updateSQL: (sql: string) => void;
  importSQL: (sql: string) => { success: boolean; errors: string[] };
  getSQLValidation: (sql: string) => { valid: boolean; errors: string[] };

  save: () => void;
  debouncedSave: () => void;
  setConfig: (config: Partial<StudioConfig>) => void;

  // Undo/Redo
  pushHistory: () => void;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;

  // Schema detection
  getUpstreamSchema: (elementId: string) => UpstreamAttribute[];
  getAllStreams: () => { id: string; name: string; attributes: UpstreamAttribute[] }[];
}

const defaultConfig: StudioConfig = {
  gridSize: 20,
  snapToGrid: true,
  autoSave: true,
  engineHost: 'localhost',
  enginePort: 9090,
};

// Store reference for debounced save - will be set after store creation
let saveToVSCode: (() => void) | null = null;

// Debounced save function (500ms delay to batch rapid changes)
const debouncedSave = debounce(() => {
  if (saveToVSCode) saveToVSCode();
}, 500);

export const useApplicationStore = create<ApplicationState>()(
  subscribeWithSelector((set, get) => ({
    // Initial state
    application: {
      elements: [],
      connections: [],
    },
    name: 'Untitled Project',
    isDirty: false,
    nodes: [],
    edges: [],
    selectedElementIds: [],
    selectedConnectionId: null,
    viewMode: 'visual',
    generatedSQL: '',
    config: defaultConfig,
    past: [],
    future: [],

    // Load application from VS Code
    loadApplication: (data) => {
      const nodes: Node[] = data.application.elements.map((el) => ({
        id: el.id,
        type: el.type,
        position: el.position,
        data: el.properties as unknown as Record<string, unknown>,
      }));

      const edges: Edge[] = data.application.connections.map((conn) => ({
        id: conn.id,
        source: conn.sourceElementId,
        sourceHandle: conn.sourcePortId,
        target: conn.targetElementId,
        targetHandle: conn.targetPortId,
        animated: true,
      }));

      set({
        application: data.application,
        name: data.name,
        nodes,
        edges,
        isDirty: false,
        past: [],
        future: [],
      });

      get().regenerateSQL();
    },

    // React Flow node/edge state
    setNodes: (nodes) => {
      const elements: VisualElement[] = nodes.map((node) => ({
        id: node.id,
        type: node.type as ElementType,
        position: node.position as { x: number; y: number },
        properties: node.data as unknown as ElementProperties,
      }));

      set((state) => ({
        nodes,
        application: {
          ...state.application,
          elements,
        },
        isDirty: true,
      }));

      get().regenerateSQL();
      if (get().config.autoSave) {
        get().debouncedSave();
      }
    },

    setEdges: (edges) => {
      const connections = edges.map((edge) => ({
        id: edge.id,
        sourceElementId: edge.source,
        sourcePortId: edge.sourceHandle || 'output',
        targetElementId: edge.target,
        targetPortId: edge.targetHandle || 'input',
      }));

      set((state) => ({
        edges,
        application: {
          ...state.application,
          connections,
        },
        isDirty: true,
      }));

      get().regenerateSQL();
      if (get().config.autoSave) {
        get().debouncedSave();
      }
    },

    onNodesChange: (changes) => {
      set((state) => ({
        nodes: applyNodeChanges(changes, state.nodes),
      }));
    },

    onEdgesChange: (changes) => {
      set((state) => ({
        edges: applyEdgeChanges(changes, state.edges),
      }));
    },

    // Element management
    addElement: (type, position, properties = {}) => {
      // Push current state to history before making changes
      get().pushHistory();

      const id = `${type}-${Date.now()}`;
      const data = { ...getDefaultProperties(type), ...properties };
      const newNode: Node = {
        id,
        type,
        position,
        data,
      };
      const newElement: VisualElement = {
        id,
        type,
        position,
        properties: data as unknown as ElementProperties,
      };

      set((state) => ({
        nodes: [...state.nodes, newNode],
        application: {
          ...state.application,
          elements: [...state.application.elements, newElement],
        },
        isDirty: true,
      }));

      get().regenerateSQL();
      if (get().config.autoSave) {
        get().debouncedSave();
      }
    },

    removeElement: (id) => {
      // Push current state to history before making changes
      get().pushHistory();

      set((state) => ({
        nodes: state.nodes.filter((n) => n.id !== id),
        edges: state.edges.filter((e) => e.source !== id && e.target !== id),
        application: {
          elements: state.application.elements.filter((el) => el.id !== id),
          connections: state.application.connections.filter(
            (c) => c.sourceElementId !== id && c.targetElementId !== id
          ),
        },
        selectedElementIds: state.selectedElementIds.filter((i) => i !== id),
        isDirty: true,
      }));

      get().regenerateSQL();
      if (get().config.autoSave) {
        get().debouncedSave();
      }
    },

    updateElement: (id, changes) => {
      set((state) => {
        const updatedNodes = state.nodes.map((node) => {
          if (node.id !== id) return node;

          // Handle data updates (from properties panel)
          const nodeChanges = changes as { data?: Record<string, unknown> };
          if (nodeChanges.data) {
            return {
              ...node,
              data: { ...node.data, ...nodeChanges.data },
            };
          }

          // Handle other changes
          return { ...node, ...changes };
        });

        // Sync application.elements
        const updatedElements = state.application.elements.map((el) => {
          if (el.id !== id) return el;
          const node = updatedNodes.find((n) => n.id === id);
          if (!node) return el;
          return {
            ...el,
            position: node.position as { x: number; y: number },
            properties: node.data as unknown as ElementProperties,
          };
        });

        return {
          nodes: updatedNodes,
          application: {
            ...state.application,
            elements: updatedElements,
          },
          isDirty: true,
        };
      });

      get().regenerateSQL();
      if (get().config.autoSave) {
        get().debouncedSave();
      }
    },

    // Connection management
    addConnection: (connection) => {
      // Push current state to history before making changes
      get().pushHistory();

      const edge: Edge = {
        id: connection.id,
        source: connection.sourceElementId,
        sourceHandle: connection.sourcePortId,
        target: connection.targetElementId,
        targetHandle: connection.targetPortId,
        animated: true,
      };

      set((state) => ({
        edges: [...state.edges, edge],
        isDirty: true,
      }));

      get().regenerateSQL();
    },

    removeConnection: (id) => {
      // Push current state to history before making changes
      get().pushHistory();

      set((state) => ({
        edges: state.edges.filter((e) => e.id !== id),
        selectedConnectionId: state.selectedConnectionId === id ? null : state.selectedConnectionId,
        isDirty: true,
      }));

      get().regenerateSQL();
    },

    // Selection
    selectElement: (id, multi = false) => {
      set((state) => {
        if (id === null) {
          return { selectedElementIds: [] };
        }
        if (multi) {
          const isSelected = state.selectedElementIds.includes(id);
          return {
            selectedElementIds: isSelected
              ? state.selectedElementIds.filter((i) => i !== id)
              : [...state.selectedElementIds, id],
          };
        }
        return { selectedElementIds: [id] };
      });
    },

    clearSelection: () => {
      set({ selectedElementIds: [], selectedConnectionId: null });
    },

    // View mode
    setViewMode: (mode) => {
      set({ viewMode: mode });
    },

    // SQL generation
    regenerateSQL: () => {
      const { application } = get();
      const sql = generateSQL(application);
      set({ generatedSQL: sql });
    },

    // SQL sync - update SQL directly (for manual editing)
    updateSQL: (sql: string) => {
      set({ generatedSQL: sql });
    },

    // Import SQL and convert to visual elements
    importSQL: (sql: string) => {
      const { elements, errors } = parseSQL(sql);

      if (errors.length > 0) {
        return { success: false, errors };
      }

      if (elements.length === 0) {
        return { success: false, errors: ['No valid elements found in SQL'] };
      }

      // Convert VisualElements to Nodes
      const nodes: Node[] = elements.map((el) => ({
        id: el.id,
        type: el.type,
        position: el.position,
        data: el.properties as unknown as Record<string, unknown>,
      }));

      // Create auto-connections between sequential elements
      const edges: Edge[] = [];
      for (let i = 0; i < elements.length - 1; i++) {
        const current = elements[i];
        const next = elements[i + 1];

        // Skip connecting unrelated element types
        if (['stream', 'table', 'trigger'].includes(current.type) &&
            ['stream', 'table', 'trigger'].includes(next.type)) {
          continue;
        }

        // Connect processing elements in sequence
        edges.push({
          id: `edge-${Date.now()}-${i}`,
          source: current.id,
          target: next.id,
          sourceHandle: 'output',
          targetHandle: 'input',
          animated: true,
        });
      }

      set({
        nodes,
        edges,
        application: {
          elements,
          connections: edges.map((e) => ({
            id: e.id,
            sourceElementId: e.source,
            sourcePortId: e.sourceHandle || 'output',
            targetElementId: e.target,
            targetPortId: e.targetHandle || 'input',
          })),
        },
        generatedSQL: sql,
        isDirty: true,
      });

      if (get().config.autoSave) {
        get().debouncedSave();
      }

      return { success: true, errors: [] };
    },

    // Validate SQL without importing
    getSQLValidation: (sql: string) => {
      return validateSQL(sql);
    },

    // Save - actual save implementation
    save: () => {
      const state = get();
      const elements: VisualElement[] = state.nodes.map((node) => ({
        id: node.id,
        type: node.type as ElementType,
        position: node.position as { x: number; y: number },
        properties: node.data as unknown as ElementProperties,
      }));

      const data: VisualApplication = {
        $schema: 'https://eventflux.io/schemas/studio/v1.json',
        version: '1.0',
        name: state.name,
        application: {
          elements,
          connections: state.edges.map((edge) => ({
            id: edge.id,
            sourceElementId: edge.source,
            sourcePortId: edge.sourceHandle || 'output',
            targetElementId: edge.target,
            targetPortId: edge.targetHandle || 'input',
          })),
        },
        layout: {
          zoom: 1,
          pan: { x: 0, y: 0 },
          gridSize: state.config.gridSize,
          snapToGrid: state.config.snapToGrid,
        },
        metadata: {
          created: new Date().toISOString(),
          modified: new Date().toISOString(),
        },
      };

      vscode.postMessage({ type: 'update', content: data });
      set({ isDirty: false });
    },

    // Debounced save for autoSave
    debouncedSave: () => {
      // Set the reference for debounced save if not set
      if (!saveToVSCode) {
        saveToVSCode = () => get().save();
      }
      debouncedSave();
    },

    // Configuration
    setConfig: (config) => {
      set((state) => ({
        config: { ...state.config, ...config },
      }));
    },

    // Schema detection - get available attributes from upstream elements
    getUpstreamSchema: (elementId: string): UpstreamAttribute[] => {
      const { nodes, edges } = get();
      const attributes: UpstreamAttribute[] = [];
      const visited = new Set<string>();

      // Find all upstream elements through connections
      const findUpstreamElements = (nodeId: string): Node[] => {
        if (visited.has(nodeId)) return [];
        visited.add(nodeId);

        const result: Node[] = [];
        const incomingEdges = edges.filter((e) => e.target === nodeId);

        for (const edge of incomingEdges) {
          const sourceNode = nodes.find((n) => n.id === edge.source);
          if (sourceNode) {
            result.push(sourceNode);
            result.push(...findUpstreamElements(sourceNode.id));
          }
        }

        return result;
      };

      const upstreamNodes = findUpstreamElements(elementId);

      // Extract attributes from upstream elements
      for (const node of upstreamNodes) {
        const data = node.data as Record<string, unknown>;

        switch (node.type) {
          case 'stream': {
            const streamName = (data.streamName as string) || '';
            const attrs = (data.attributes as { name: string; type: string }[]) || [];
            for (const attr of attrs) {
              attributes.push({
                name: attr.name,
                type: attr.type,
                streamId: streamName,
                source: node.id,
              });
            }
            break;
          }
          case 'table': {
            const tableName = (data.tableName as string) || '';
            const attrs = (data.attributes as { name: string; type: string }[]) || [];
            for (const attr of attrs) {
              attributes.push({
                name: attr.name,
                type: attr.type,
                streamId: tableName,
                source: node.id,
              });
            }
            break;
          }
          case 'projection': {
            // Projections can introduce aliases
            const selectList = (data.selectList as { expression: Record<string, unknown>; alias?: string }[]) || [];
            for (const item of selectList) {
              const alias = item.alias;
              const expr = item.expression;
              if (alias) {
                // Get type from expression if possible, default to STRING
                let attrType = 'STRING';
                if (expr?.type === 'variable' && expr?.variableName) {
                  // Find original attribute type
                  const origAttr = attributes.find((a) => a.name === expr.variableName);
                  if (origAttr) attrType = origAttr.type;
                }
                attributes.push({
                  name: alias,
                  type: attrType,
                  source: node.id,
                });
              }
            }
            break;
          }
          case 'aggregation': {
            // Aggregations create new attributes
            const aggs = (data.aggregations as { type: string; alias: string }[]) || [];
            for (const agg of aggs) {
              // Determine result type based on aggregation
              let resultType = 'DOUBLE';
              if (agg.type === 'COUNT' || agg.type === 'DISTINCTCOUNT') {
                resultType = 'LONG';
              } else if (agg.type === 'FIRST' || agg.type === 'LAST') {
                // Would need to look at input expression type
                resultType = 'STRING';
              }
              attributes.push({
                name: agg.alias,
                type: resultType,
                source: node.id,
              });
            }
            break;
          }
        }
      }

      return attributes;
    },

    // Get all defined streams and tables
    getAllStreams: () => {
      const { nodes } = get();
      const result: { id: string; name: string; attributes: UpstreamAttribute[] }[] = [];

      for (const node of nodes) {
        const data = node.data as Record<string, unknown>;

        if (node.type === 'stream') {
          const streamName = (data.streamName as string) || '';
          const attrs = (data.attributes as { name: string; type: string }[]) || [];
          result.push({
            id: node.id,
            name: streamName,
            attributes: attrs.map((a) => ({
              name: a.name,
              type: a.type,
              streamId: streamName,
              source: node.id,
            })),
          });
        } else if (node.type === 'table') {
          const tableName = (data.tableName as string) || '';
          const attrs = (data.attributes as { name: string; type: string }[]) || [];
          result.push({
            id: node.id,
            name: tableName,
            attributes: attrs.map((a) => ({
              name: a.name,
              type: a.type,
              streamId: tableName,
              source: node.id,
            })),
          });
        }
      }

      return result;
    },

    // Undo/Redo implementation using past/future stacks
    pushHistory: () => {
      const { nodes, edges, past } = get();
      // Save current state to past
      const snapshot: HistorySnapshot = {
        nodes: JSON.parse(JSON.stringify(nodes)),
        edges: JSON.parse(JSON.stringify(edges)),
      };
      // Add to past, clear future (new action invalidates redo)
      const newPast = [...past, snapshot];
      // Limit history to 50 states
      if (newPast.length > 50) {
        newPast.shift();
      }
      set({ past: newPast, future: [] });
    },

    undo: () => {
      const { nodes, edges, past, future } = get();
      if (past.length === 0) return;

      // Save current state to future (for redo)
      const currentSnapshot: HistorySnapshot = {
        nodes: JSON.parse(JSON.stringify(nodes)),
        edges: JSON.parse(JSON.stringify(edges)),
      };

      // Pop from past and restore
      const newPast = [...past];
      const previousState = newPast.pop()!;

      set({
        nodes: JSON.parse(JSON.stringify(previousState.nodes)),
        edges: JSON.parse(JSON.stringify(previousState.edges)),
        past: newPast,
        future: [...future, currentSnapshot],
        isDirty: true,
      });

      // Regenerate SQL after undo
      setTimeout(() => get().regenerateSQL(), 0);
    },

    redo: () => {
      const { nodes, edges, past, future } = get();
      if (future.length === 0) return;

      // Save current state to past
      const currentSnapshot: HistorySnapshot = {
        nodes: JSON.parse(JSON.stringify(nodes)),
        edges: JSON.parse(JSON.stringify(edges)),
      };

      // Pop from future and restore
      const newFuture = [...future];
      const nextState = newFuture.pop()!;

      set({
        nodes: JSON.parse(JSON.stringify(nextState.nodes)),
        edges: JSON.parse(JSON.stringify(nextState.edges)),
        past: [...past, currentSnapshot],
        future: newFuture,
        isDirty: true,
      });

      // Regenerate SQL after redo
      setTimeout(() => get().regenerateSQL(), 0);
    },

    canUndo: () => get().past.length > 0,
    canRedo: () => get().future.length > 0,
  }))
);

// Helper functions
function getDefaultProperties(type: ElementType): Record<string, unknown> {
  switch (type) {
    case 'source': {
      // Default to websocket, fallback to first available from schema
      const sourceTypes = getSourceTypes();
      const defaultSourceType = sourceTypes.includes('websocket') ? 'websocket' : sourceTypes[0] || 'timer';
      const schema = getSourceSchema(defaultSourceType);
      // Initialize config with required parameters
      const config: Record<string, string> = {};
      if (schema?.requiredParameters) {
        for (const param of schema.requiredParameters) {
          config[param] = '';
        }
      }
      return {
        sourceName: 'NewSource',
        sourceType: defaultSourceType,
        config,
      };
    }
    case 'sink': {
      // Get first available sink type from schema
      const sinkTypes = getSinkTypes();
      const defaultSinkType = sinkTypes[0] || 'log';
      const schema = getSinkSchema(defaultSinkType);
      // Initialize config with required parameters
      const config: Record<string, string> = {};
      if (schema?.requiredParameters) {
        for (const param of schema.requiredParameters) {
          config[param] = '';
        }
      }
      return {
        sinkName: 'NewSink',
        sinkType: defaultSinkType,
        config,
      };
    }
    case 'stream':
      return {
        streamName: 'NewStream',
        attributes: [
          { name: 'id', type: 'INT' },
          { name: 'value', type: 'DOUBLE' },
        ],
      };
    case 'table':
      return {
        tableName: 'NewTable',
        attributes: [
          { name: 'key', type: 'STRING' },
          { name: 'value', type: 'STRING' },
        ],
      };
    case 'trigger':
      return {
        triggerId: 'NewTrigger',
        triggerType: 'periodic',
        atEvery: 1000,
      };
    case 'window': {
      // Get first available window type from schema
      const windowTypes = getWindowTypes();
      const defaultWindowType = windowTypes.includes('length') ? 'length' : windowTypes[0] || 'length';
      return {
        windowType: defaultWindowType,
        parameters: { count: 10 },
      };
    }
    case 'filter':
      return {
        condition: null,
      };
    case 'projection':
      return {
        selectList: [],
      };
    case 'aggregation':
      return {
        aggregations: [],
      };
    case 'groupBy':
      return {
        groupByAttributes: [],
      };
    case 'join':
      return {
        joinType: 'inner',
        onCondition: null,
      };
    case 'pattern':
      return {
        mode: 'pattern',
        patternExpression: { type: 'stream', streamName: '' },
      };
    case 'partition':
      return {
        partitionBy: [],
      };
    default:
      return {};
  }
}

function generateSQL(application: { elements: VisualElement[]; connections: Connection[] }): string {
  const lines: string[] = [];
  const { elements, connections } = application;

  // Find streams and generate CREATE STREAM statements
  const streams = elements.filter((el) => el.type === 'stream');
  for (const stream of streams) {
    const props = stream.properties as { streamName: string; attributes: { name: string; type: string }[] };
    const attrs = props.attributes?.map((a) => `${a.name} ${a.type}`).join(', ') || '';
    lines.push(`CREATE STREAM ${props.streamName} (${attrs});`);
  }

  // Find tables and generate CREATE TABLE statements
  const tables = elements.filter((el) => el.type === 'table');
  for (const table of tables) {
    const props = table.properties as { tableName: string; attributes: { name: string; type: string }[] };
    const attrs = props.attributes?.map((a) => `${a.name} ${a.type}`).join(', ') || '';
    lines.push(`CREATE TABLE ${props.tableName} (${attrs});`);
  }

  // Find triggers and generate CREATE TRIGGER statements
  const triggers = elements.filter((el) => el.type === 'trigger');
  for (const trigger of triggers) {
    const props = trigger.properties as { triggerId: string; triggerType: string; atEvery?: number; cronExpression?: string };
    if (props.triggerType === 'start') {
      lines.push(`CREATE TRIGGER ${props.triggerId} AT START;`);
    } else if (props.triggerType === 'periodic') {
      lines.push(`CREATE TRIGGER ${props.triggerId} AT EVERY ${props.atEvery} MILLISECONDS;`);
    } else if (props.triggerType === 'cron') {
      lines.push(`CREATE TRIGGER ${props.triggerId} AT CRON '${props.cronExpression}';`);
    }
  }

  if (lines.length > 0) {
    lines.push('');
  }

  // Find streams that are targets of processing elements (INSERT INTO targets)
  // A stream is a target if it receives connections from processing elements (not sources)
  const processingTypes = ['window', 'filter', 'projection', 'aggregation', 'groupBy', 'join', 'pattern', 'partition'];

  let queryCount = 0;
  for (const conn of connections) {
    const sourceEl = elements.find((el) => el.id === conn.sourceElementId);
    const targetEl = elements.find((el) => el.id === conn.targetElementId);

    // Check if this is a processing element connecting to a stream (INSERT INTO pattern)
    if (sourceEl && targetEl && processingTypes.includes(sourceEl.type) && targetEl.type === 'stream') {
      const targetStreamName = (targetEl.properties as { streamName: string }).streamName;
      const query = buildQueryFromProcessingToStream(sourceEl, targetStreamName, elements, connections);
      if (query) {
        // Add blank line between queries
        if (queryCount > 0) {
          lines.push('');
        }
        lines.push(query);
        queryCount++;
      }
    }
  }

  // Also handle simple stream-to-sink flows
  // If a stream connects directly to a sink, we may need to log that differently
  // For now, we only generate INSERT INTO queries for processing-to-stream connections

  return lines.join('\n');
}

// Build a query from a processing element that connects to a target stream (INSERT INTO)
function buildQueryFromProcessingToStream(
  processingElement: VisualElement,
  targetStreamName: string,
  elements: VisualElement[],
  connections: Connection[]
): string | null {
  // Trace back through the pipeline from the processing element
  const pipeline = traceBackPipeline(processingElement.id, elements, connections);
  if (pipeline.length === 0) return null;

  // Build the query from the pipeline
  let fromClause = '';
  let windowClause = '';
  let whereClause = '';
  let selectClause = 'SELECT *';
  let groupByClause = '';
  let havingClause = '';

  for (const el of pipeline) {
    switch (el.type) {
      case 'stream': {
        const props = el.properties as { streamName: string };
        fromClause = props.streamName;
        break;
      }
      case 'window': {
        const props = el.properties as { windowType: string; parameters: Record<string, unknown> };
        windowClause = generateWindowClause(props);
        break;
      }
      case 'filter': {
        const props = el.properties as { condition: unknown };
        if (props.condition) {
          whereClause = `WHERE ${expressionToSQL(props.condition)}`;
        }
        break;
      }
      case 'projection': {
        const props = el.properties as { selectList: { expression: unknown; alias?: string }[] };
        if (props.selectList && props.selectList.length > 0) {
          selectClause = 'SELECT ' + props.selectList
            .map((item) => {
              const expr = expressionToSQL(item.expression);
              return item.alias ? `${expr} AS ${item.alias}` : expr;
            })
            .join(', ');
        }
        break;
      }
      case 'aggregation': {
        const props = el.properties as { aggregations: { type: string; expression: unknown; alias: string }[] };
        if (props.aggregations && props.aggregations.length > 0) {
          const aggParts = props.aggregations.map((agg) => {
            const expr = expressionToSQL(agg.expression);
            return `${agg.type}(${expr}) AS ${agg.alias}`;
          });
          selectClause = 'SELECT ' + aggParts.join(', ');
        }
        break;
      }
      case 'groupBy': {
        const props = el.properties as { groupByAttributes: string[]; havingCondition?: unknown };
        if (props.groupByAttributes && props.groupByAttributes.length > 0) {
          groupByClause = `GROUP BY ${props.groupByAttributes.join(', ')}`;
        }
        if (props.havingCondition) {
          havingClause = `HAVING ${expressionToSQL(props.havingCondition)}`;
        }
        break;
      }
    }
  }

  if (!fromClause) return null;

  const parts = [
    `INSERT INTO ${targetStreamName}`,
    selectClause,
    `FROM ${fromClause}${windowClause ? ' ' + windowClause : ''}`,
  ];

  if (whereClause) parts.push(whereClause);
  if (groupByClause) parts.push(groupByClause);
  if (havingClause) parts.push(havingClause);

  return parts.join('\n') + ';';
}

function traceBackPipeline(
  elementId: string,
  elements: VisualElement[],
  connections: Connection[]
): VisualElement[] {
  const result: VisualElement[] = [];
  let currentId: string | null = elementId;

  while (currentId) {
    const element = elements.find((el) => el.id === currentId);
    if (!element) break;

    result.unshift(element);

    // Find input connection
    const inputConn = connections.find((c) => c.targetElementId === currentId);
    currentId = inputConn ? inputConn.sourceElementId : null;
  }

  return result;
}

function generateWindowClause(props: { windowType: string; parameters: Record<string, unknown> }): string {
  const { windowType, parameters } = props;

  switch (windowType) {
    case 'length':
      return `WINDOW('length', ${parameters.count})`;
    case 'lengthBatch':
      return `WINDOW('lengthBatch', ${parameters.count})`;
    case 'time':
      return `WINDOW('time', ${formatDuration(parameters.duration as { value: number; unit: string })})`;
    case 'timeBatch':
      return `WINDOW('timeBatch', ${formatDuration(parameters.duration as { value: number; unit: string })})`;
    case 'tumbling':
      return `WINDOW('tumbling', ${formatDuration(parameters.duration as { value: number; unit: string })})`;
    case 'sliding':
      return `WINDOW('sliding', ${formatDuration(parameters.duration as { value: number; unit: string })}, ${formatDuration(parameters.slideInterval as { value: number; unit: string })})`;
    case 'session':
      return `WINDOW('session', ${formatDuration(parameters.gapDuration as { value: number; unit: string })})`;
    case 'externalTime':
      return `WINDOW('externalTime', ${parameters.timestampAttribute}, ${formatDuration(parameters.duration as { value: number; unit: string })})`;
    case 'externalTimeBatch':
      return `WINDOW('externalTimeBatch', ${parameters.timestampAttribute}, ${formatDuration(parameters.duration as { value: number; unit: string })})`;
    case 'sort':
      return `WINDOW('sort', ${parameters.count}, ${parameters.sortAttribute})`;
    default:
      return '';
  }
}

function formatDuration(duration: { value: number; unit: string } | undefined): string {
  if (!duration) return '0 SECONDS';
  return `${duration.value} ${duration.unit}`;
}

function expressionToSQL(expr: unknown): string {
  if (!expr) return '*';
  if (typeof expr !== 'object') return String(expr);

  const e = expr as Record<string, unknown>;
  switch (e.type) {
    case 'constant':
      if (e.constantType === 'string') return `'${e.constantValue}'`;
      if (e.constantType === 'null') return 'NULL';
      return String(e.constantValue);

    case 'variable':
      return e.streamId ? `${e.streamId}.${e.variableName}` : String(e.variableName);

    case 'function':
      const params = (e.parameters as unknown[] || []).map((p) => expressionToSQL(p)).join(', ');
      return `${String(e.functionName).toUpperCase()}(${params})`;

    case 'compare':
      return `${expressionToSQL(e.left)} ${e.operator} ${expressionToSQL(e.right)}`;

    case 'and':
      return `(${expressionToSQL(e.left)} AND ${expressionToSQL(e.right)})`;

    case 'or':
      return `(${expressionToSQL(e.left)} OR ${expressionToSQL(e.right)})`;

    case 'not':
      return `NOT ${expressionToSQL(e.operand)}`;

    case 'add':
      return `(${expressionToSQL(e.left)} + ${expressionToSQL(e.right)})`;

    case 'subtract':
      return `(${expressionToSQL(e.left)} - ${expressionToSQL(e.right)})`;

    case 'multiply':
      return `(${expressionToSQL(e.left)} * ${expressionToSQL(e.right)})`;

    case 'divide':
      return `(${expressionToSQL(e.left)} / ${expressionToSQL(e.right)})`;

    default:
      return '*';
  }
}
