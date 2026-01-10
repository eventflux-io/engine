import { useCallback, useEffect, useRef, useState } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  Connection,
  BackgroundVariant,
  ReactFlowProvider,
  useNodesState,
  useEdgesState,
  Node,
  Edge,
  addEdge,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { useApplicationStore } from './stores/applicationStore';
import { Toolbar } from './components/Toolbar/Toolbar';
import { Palette } from './components/Palette/Palette';
import { PropertiesPanel } from './components/Properties/PropertiesPanel';
import { ConfigPanel } from './components/ConfigPanel/ConfigPanel';
import { SQLEditor } from './components/SQLEditor/SQLEditor';
import { nodeTypes } from './elements/nodeTypes';
import { vscode } from './utils/vscode';
import { validateConnection } from './utils/connectionRules';

function Flow({ onConnectionError }: { onConnectionError: (message: string) => void }) {
  const {
    selectElement,
    setNodes: setStoreNodes,
    setEdges: setStoreEdges,
    nodes: storeNodes,
    edges: storeEdges,
  } = useApplicationStore();

  // Use React Flow's built-in state management - initialize from store
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>(storeNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>(storeEdges);

  // Track sync direction to prevent infinite loops
  const syncingToStore = useRef(false);
  const lastStoreUpdate = useRef<string>('');

  // Sync to store when nodes change (for SQL generation and properties panel)
  useEffect(() => {
    // Create a signature of current nodes to detect actual changes
    const nodeSignature = nodes.map(n => `${n.id}:${n.position.x}:${n.position.y}`).join(',');
    const edgeSignature = edges.map(e => `${e.id}:${e.source}:${e.target}`).join(',');
    const currentSignature = `${nodeSignature}|${edgeSignature}`;

    // Only sync if there's an actual change and we're not already syncing
    if (currentSignature !== lastStoreUpdate.current) {
      syncingToStore.current = true;
      lastStoreUpdate.current = currentSignature;
      setStoreNodes(nodes);
      setStoreEdges(edges);
      // Reset after a tick to allow store update to complete
      setTimeout(() => {
        syncingToStore.current = false;
      }, 0);
    }
  }, [nodes, edges, setStoreNodes, setStoreEdges]);

  // Sync property changes from store back to local state (only for data updates)
  useEffect(() => {
    // Skip if we just synced to store or if store is empty
    if (syncingToStore.current || storeNodes.length === 0) return;

    setNodes((currentNodes) => {
      let hasChanges = false;
      const updated = currentNodes.map((node) => {
        const storeNode = storeNodes.find((n) => n.id === node.id);
        if (storeNode && JSON.stringify(node.data) !== JSON.stringify(storeNode.data)) {
          hasChanges = true;
          return { ...node, data: storeNode.data };
        }
        return node;
      });
      return hasChanges ? updated : currentNodes;
    });
  }, [storeNodes, setNodes]);

  const onConnect = useCallback(
    (params: Connection) => {
      // Validate connection before adding
      const validation = validateConnection(params, nodes, edges);
      if (!validation.valid) {
        onConnectionError(validation.reason || 'Invalid connection');
        return;
      }
      setEdges((eds) => addEdge({ ...params, animated: true }, eds));
    },
    [setEdges, nodes, edges, onConnectionError]
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      selectElement(node.id);
    },
    [selectElement]
  );

  const onPaneClick = useCallback(() => {
    selectElement(null);
  }, [selectElement]);

  const onEdgeClick = useCallback(
    (_: React.MouseEvent, edge: Edge) => {
      // Deselect nodes and select this edge
      selectElement(null);
      // Visually select the edge by updating its style
      setEdges((eds) =>
        eds.map((e) => ({
          ...e,
          selected: e.id === edge.id,
        }))
      );
    },
    [selectElement, setEdges]
  );

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const type = event.dataTransfer.getData('application/reactflow');
      if (!type) return;

      const reactFlowBounds = event.currentTarget.getBoundingClientRect();
      const position = {
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      };

      const newNode: Node = {
        id: `${type}-${Date.now()}`,
        type,
        position,
        data: getDefaultProperties(type, nodes),
      };

      setNodes((nds) => [...nds, newNode]);
    },
    [setNodes, nodes]
  );

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  return (
    <div className="w-full h-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        onEdgeClick={onEdgeClick}
        onPaneClick={onPaneClick}
        onDrop={onDrop}
        onDragOver={onDragOver}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{
          maxZoom: 0.8,
          minZoom: 0.3,
          padding: 0.2,
        }}
        minZoom={0.2}
        maxZoom={2}
        snapToGrid
        snapGrid={[20, 20]}
        deleteKeyCode={['Backspace', 'Delete']}
        defaultEdgeOptions={{
          animated: true,
          style: { stroke: '#6366f1', strokeWidth: 2 },
        }}
      >
        <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#444" />
        <Controls />
        <MiniMap
          nodeStrokeWidth={3}
          pannable
          zoomable
          style={{ backgroundColor: '#1e1e1e' }}
        />
      </ReactFlow>
    </div>
  );
}

function App() {
  const { loadApplication, setConfig, viewMode } = useApplicationStore();
  const [rightPanel, setRightPanel] = useState<'properties' | 'config'>('properties');
  const [toast, setToast] = useState<{ message: string; type: 'error' | 'success' } | null>(null);

  const showToast = useCallback((message: string, type: 'error' | 'success' = 'error') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  }, []);

  const handleConnectionError = useCallback((message: string) => {
    showToast(message, 'error');
  }, [showToast]);

  useEffect(() => {
    // Listen for messages from VS Code
    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      switch (message.type) {
        case 'load':
          if (message.data) {
            loadApplication(message.data);
          }
          break;
        case 'config':
          setConfig(message.config);
          break;
        case 'error':
          console.error('VS Code error:', message.message);
          break;
      }
    };

    window.addEventListener('message', handleMessage);

    // Tell VS Code we're ready
    vscode.postMessage({ type: 'ready' });
    vscode.postMessage({ type: 'getConfig' });

    return () => {
      window.removeEventListener('message', handleMessage);
    };
  }, [loadApplication, setConfig]);

  return (
    <ReactFlowProvider>
      <div className="h-screen flex flex-col bg-vscode-bg text-vscode-fg">
        <Toolbar />
        <div className="flex-1 flex overflow-hidden">
          {/* Palette - only show in visual or split mode */}
          {viewMode !== 'sql' && <Palette />}

          {/* Main content area */}
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Visual canvas - always mounted, hidden when in SQL-only mode */}
            <div
              className={`${viewMode === 'split' ? 'h-1/2' : 'flex-1'} relative`}
              style={{
                minHeight: '300px',
                display: viewMode === 'sql' ? 'none' : 'block',
              }}
            >
              <Flow onConnectionError={handleConnectionError} />

              {/* Toast notification */}
              {toast && (
                <div
                  className={`absolute top-4 left-1/2 -translate-x-1/2 px-4 py-2 rounded-lg shadow-lg z-50 text-sm font-medium transition-opacity ${
                    toast.type === 'error'
                      ? 'bg-red-600 text-white'
                      : 'bg-green-600 text-white'
                  }`}
                >
                  {toast.message}
                </div>
              )}
            </div>

            {/* SQL Editor - show in sql or split mode */}
            {viewMode !== 'visual' && <SQLEditor />}
          </div>

          {/* Right panel - only show in visual or split mode */}
          {viewMode !== 'sql' && (
            <div className="w-72 border-l border-vscode-border bg-gray-900/30 flex flex-col">
              {/* Panel tabs */}
              <div className="flex border-b border-vscode-border">
                <button
                  onClick={() => setRightPanel('properties')}
                  className={`flex-1 px-3 py-2 text-xs font-medium transition-colors ${
                    rightPanel === 'properties'
                      ? 'text-white bg-gray-800/50 border-b-2 border-indigo-500'
                      : 'text-gray-400 hover:text-white hover:bg-gray-800/30'
                  }`}
                >
                  Properties
                </button>
                <button
                  onClick={() => setRightPanel('config')}
                  className={`flex-1 px-3 py-2 text-xs font-medium transition-colors ${
                    rightPanel === 'config'
                      ? 'text-white bg-gray-800/50 border-b-2 border-indigo-500'
                      : 'text-gray-400 hover:text-white hover:bg-gray-800/30'
                  }`}
                >
                  Config
                </button>
              </div>

              {/* Panel content */}
              <div className="flex-1 overflow-hidden">
                {rightPanel === 'properties' ? (
                  <PropertiesPanelContent />
                ) : (
                  <ConfigPanel />
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </ReactFlowProvider>
  );
}

// Extract PropertiesPanel content (without outer wrapper since we're using shared wrapper)
function PropertiesPanelContent() {
  return <PropertiesPanel />;
}

// Helper to generate unique names for elements
function getNextNumber(nodes: Node[], type: string, nameField: string): number {
  const pattern = new RegExp(`^${type.charAt(0).toUpperCase() + type.slice(1)}(\\d+)$`);
  let maxNum = 0;

  for (const node of nodes) {
    if (node.type === type) {
      const name = (node.data as Record<string, unknown>)[nameField] as string;
      if (name) {
        const match = name.match(pattern);
        if (match) {
          maxNum = Math.max(maxNum, parseInt(match[1], 10));
        }
      }
    }
  }

  return maxNum + 1;
}

function getDefaultProperties(type: string, nodes: Node[] = []): Record<string, unknown> {
  const num = (nameField: string) => getNextNumber(nodes, type, nameField);

  switch (type) {
    case 'source':
      return {
        sourceName: `Source${num('sourceName')}`,
        sourceType: 'kafka',
        config: {
          'bootstrap.servers': 'localhost:9092',
          'topic': 'events',
          'group.id': 'eventflux-consumer',
        },
      };
    case 'sink':
      return {
        sinkName: `Sink${num('sinkName')}`,
        sinkType: 'log',
        config: {},
      };
    case 'stream':
      return {
        streamName: `Stream${num('streamName')}`,
        attributes: [
          { name: 'id', type: 'INT' },
          { name: 'value', type: 'DOUBLE' },
        ],
      };
    case 'table':
      return {
        tableName: `Table${num('tableName')}`,
        attributes: [
          { name: 'key', type: 'STRING' },
          { name: 'value', type: 'STRING' },
        ],
      };
    case 'trigger':
      return {
        triggerId: `Trigger${num('triggerId')}`,
        triggerType: 'periodic',
        atEvery: 1000,
      };
    case 'window':
      return {
        windowType: 'length',
        parameters: { count: 10 },
      };
    case 'filter':
      return {
        condition: { type: 'compare', operator: '>', left: { type: 'variable', variableName: 'value' }, right: { type: 'constant', constantValue: 0 } },
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

export default App;
