import type { Node, Edge, Connection } from '@xyflow/react';
import type { ElementType } from '../types';

/**
 * Connection cardinality rules for each element type
 * - maxInputs: maximum number of incoming connections (null = unlimited)
 * - maxOutputs: maximum number of outgoing connections (null = unlimited)
 * - canBeSource: can this element be the starting point of a pipeline?
 * - canBeSink: can this element be the ending point of a pipeline?
 */
interface CardinalityRule {
  maxInputs: number | null;
  maxOutputs: number | null;
  canBeSource: boolean;
  canBeSink: boolean;
  allowedSources?: ElementType[]; // Which element types can connect TO this element
  allowedTargets?: ElementType[]; // Which element types this element can connect TO
}

const cardinalityRules: Record<ElementType, CardinalityRule> = {
  // External connectors
  source: {
    maxInputs: 0, // No inputs - data comes from external system
    maxOutputs: 1, // Connects to exactly one stream
    canBeSource: true,
    canBeSink: false,
    allowedTargets: ['stream'], // Can ONLY connect to streams
  },
  sink: {
    maxInputs: 1, // Receives from one stream
    maxOutputs: 0, // No outputs - data goes to external system
    canBeSource: false,
    canBeSink: true,
    allowedSources: ['stream'], // Can receive from streams (which may be query outputs via INSERT INTO)
  },

  // Data channels - Streams are bidirectional
  stream: {
    maxInputs: null, // Can receive from Source OR processing elements (INSERT INTO)
    maxOutputs: null, // Can feed many processing elements or sinks
    canBeSource: true,
    canBeSink: true, // Can be target of INSERT INTO
    allowedSources: ['source', 'window', 'filter', 'projection', 'aggregation', 'groupBy', 'join', 'pattern', 'partition'], // Source or query results
    allowedTargets: ['window', 'filter', 'projection', 'aggregation', 'groupBy', 'join', 'pattern', 'partition', 'sink', 'stream'], // Can also connect to another stream
  },
  trigger: {
    maxInputs: 0, // No inputs
    maxOutputs: null, // Can trigger many queries
    canBeSource: true,
    canBeSink: false,
    allowedTargets: ['window', 'filter', 'projection', 'aggregation', 'groupBy', 'stream'],
  },
  table: {
    maxInputs: null, // Can receive from multiple queries (writes)
    maxOutputs: null, // Can be read by multiple queries
    canBeSource: true,
    canBeSink: true,
    allowedTargets: ['window', 'filter', 'projection', 'aggregation', 'groupBy', 'join', 'stream'],
  },

  // Processing elements - typically 1:1, can output to stream (INSERT INTO)
  window: {
    maxInputs: 1,
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream', 'table', 'trigger'],
    allowedTargets: ['filter', 'projection', 'aggregation', 'groupBy', 'stream'],
  },
  filter: {
    maxInputs: 1,
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream', 'table', 'trigger', 'window'],
    allowedTargets: ['window', 'filter', 'projection', 'aggregation', 'groupBy', 'join', 'stream'],
  },
  projection: {
    maxInputs: 1,
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream', 'table', 'trigger', 'window', 'filter', 'aggregation', 'groupBy', 'join'],
    allowedTargets: ['filter', 'projection', 'stream'],
  },
  aggregation: {
    maxInputs: 1,
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream', 'table', 'trigger', 'window', 'filter', 'groupBy'],
    allowedTargets: ['projection', 'filter', 'stream'],
  },
  groupBy: {
    maxInputs: 1,
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream', 'table', 'trigger', 'window', 'filter'],
    allowedTargets: ['aggregation', 'projection', 'stream'],
  },

  // Special processing elements
  join: {
    maxInputs: 2, // Exactly 2 inputs for join
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream', 'table', 'window', 'filter'],
    allowedTargets: ['filter', 'projection', 'aggregation', 'groupBy', 'stream'],
  },
  pattern: {
    maxInputs: null, // Multiple streams can be part of pattern
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream'],
    allowedTargets: ['filter', 'projection', 'stream'],
  },
  partition: {
    maxInputs: 1,
    maxOutputs: 1,
    canBeSource: false,
    canBeSink: false,
    allowedSources: ['stream', 'window', 'filter'],
    allowedTargets: ['window', 'filter', 'projection', 'aggregation', 'groupBy', 'stream'],
  },
};

export interface ValidationResult {
  valid: boolean;
  reason?: string;
}

/**
 * Validates if a new connection is allowed based on cardinality rules
 */
export function validateConnection(
  connection: Connection,
  nodes: Node[],
  edges: Edge[]
): ValidationResult {
  const sourceNode = nodes.find((n) => n.id === connection.source);
  const targetNode = nodes.find((n) => n.id === connection.target);

  if (!sourceNode || !targetNode) {
    return { valid: false, reason: 'Source or target node not found' };
  }

  const sourceType = sourceNode.type as ElementType;
  const targetType = targetNode.type as ElementType;

  const sourceRule = cardinalityRules[sourceType];
  const targetRule = cardinalityRules[targetType];

  if (!sourceRule || !targetRule) {
    return { valid: false, reason: 'Unknown element type' };
  }

  // Check if source can have more outputs
  if (sourceRule.maxOutputs !== null) {
    const currentOutputs = edges.filter((e) => e.source === connection.source).length;
    if (currentOutputs >= sourceRule.maxOutputs) {
      return {
        valid: false,
        reason: `${getElementLabel(sourceType)} can only have ${sourceRule.maxOutputs} output connection${sourceRule.maxOutputs === 1 ? '' : 's'}`,
      };
    }
  }

  // Check if target can have more inputs
  if (targetRule.maxInputs !== null) {
    const currentInputs = edges.filter((e) => e.target === connection.target).length;
    if (currentInputs >= targetRule.maxInputs) {
      return {
        valid: false,
        reason: `${getElementLabel(targetType)} can only have ${targetRule.maxInputs} input connection${targetRule.maxInputs === 1 ? '' : 's'}`,
      };
    }
  }

  // Check if this source type is allowed to connect to this target type
  if (sourceRule.allowedTargets && !sourceRule.allowedTargets.includes(targetType)) {
    return {
      valid: false,
      reason: `${getElementLabel(sourceType)} cannot connect to ${getElementLabel(targetType)}`,
    };
  }

  // Check if this target type accepts connections from this source type
  if (targetRule.allowedSources && !targetRule.allowedSources.includes(sourceType)) {
    return {
      valid: false,
      reason: `${getElementLabel(targetType)} cannot receive connections from ${getElementLabel(sourceType)}`,
    };
  }

  // Check for duplicate connections
  const duplicateConnection = edges.find(
    (e) => e.source === connection.source && e.target === connection.target
  );
  if (duplicateConnection) {
    return { valid: false, reason: 'Connection already exists' };
  }

  // Special rule: A stream can only have ONE external connector (either Source OR Sink, not both)
  if (sourceType === 'source' && targetType === 'stream') {
    // Check if stream already has a Source
    const existingSourceConnection = edges.find((e) => {
      const edgeSourceNode = nodes.find((n) => n.id === e.source);
      return edgeSourceNode?.type === 'source' && e.target === connection.target;
    });
    if (existingSourceConnection) {
      return { valid: false, reason: 'Stream already has a Source connected.' };
    }
    // Check if stream already has a Sink
    const existingSinkConnection = edges.find((e) => {
      const edgeTargetNode = nodes.find((n) => n.id === e.target);
      return e.source === connection.target && edgeTargetNode?.type === 'sink';
    });
    if (existingSinkConnection) {
      return { valid: false, reason: 'Stream already connects to a Sink. A stream can have either a Source or a Sink, not both.' };
    }
  }

  if (sourceType === 'stream' && targetType === 'sink') {
    // Check if stream already has a Sink
    const existingSinkConnection = edges.find((e) => {
      const edgeTargetNode = nodes.find((n) => n.id === e.target);
      return e.source === connection.source && edgeTargetNode?.type === 'sink';
    });
    if (existingSinkConnection) {
      return { valid: false, reason: 'Stream already connects to a Sink.' };
    }
    // Check if stream already has a Source
    const existingSourceConnection = edges.find((e) => {
      const edgeSourceNode = nodes.find((n) => n.id === e.source);
      return edgeSourceNode?.type === 'source' && e.target === connection.source;
    });
    if (existingSourceConnection) {
      return { valid: false, reason: 'Stream already has a Source connected. A stream can have either a Source or a Sink, not both.' };
    }
  }

  // Check for self-connection
  if (connection.source === connection.target) {
    return { valid: false, reason: 'Cannot connect element to itself' };
  }

  return { valid: true };
}

/**
 * Gets the display label for an element type
 */
function getElementLabel(type: ElementType): string {
  const labels: Record<ElementType, string> = {
    source: 'Source',
    sink: 'Sink',
    stream: 'Stream',
    table: 'Table',
    trigger: 'Trigger',
    window: 'Window',
    filter: 'Filter',
    projection: 'Projection',
    aggregation: 'Aggregation',
    groupBy: 'Group By',
    join: 'Join',
    pattern: 'Pattern',
    partition: 'Partition',
  };
  return labels[type] || type;
}

/**
 * Checks if a node can accept more input connections
 */
export function canAcceptInput(nodeId: string, nodes: Node[], edges: Edge[]): boolean {
  const node = nodes.find((n) => n.id === nodeId);
  if (!node) return false;

  const rule = cardinalityRules[node.type as ElementType];
  if (!rule) return false;

  if (rule.maxInputs === null) return true;

  const currentInputs = edges.filter((e) => e.target === nodeId).length;
  return currentInputs < rule.maxInputs;
}

/**
 * Checks if a node can have more output connections
 */
export function canHaveOutput(nodeId: string, nodes: Node[], edges: Edge[]): boolean {
  const node = nodes.find((n) => n.id === nodeId);
  if (!node) return false;

  const rule = cardinalityRules[node.type as ElementType];
  if (!rule) return false;

  if (rule.maxOutputs === null) return true;

  const currentOutputs = edges.filter((e) => e.source === nodeId).length;
  return currentOutputs < rule.maxOutputs;
}

/**
 * Gets connection info for displaying in UI
 */
export function getConnectionInfo(type: ElementType): { inputs: string; outputs: string } {
  const rule = cardinalityRules[type];
  if (!rule) return { inputs: '?', outputs: '?' };

  return {
    inputs: rule.maxInputs === null ? '∞' : String(rule.maxInputs),
    outputs: rule.maxOutputs === null ? '∞' : String(rule.maxOutputs),
  };
}
