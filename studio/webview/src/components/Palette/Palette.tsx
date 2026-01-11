import { useState } from 'react';
import {
  Database,
  Table2,
  Timer,
  Clock,
  Filter,
  Columns,
  BarChart2,
  Group,
  GitMerge,
  Route,
  Layers,
  ArrowDownToLine,
  ArrowUpFromLine,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import type { ElementType } from '../../types';

interface PaletteCategory {
  id: string;
  name: string;
  description?: string;
  items: PaletteItem[];
}

interface PaletteItem {
  type: ElementType;
  name: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
}

const categories: PaletteCategory[] = [
  {
    id: 'input',
    name: 'Input',
    description: 'Data entry points',
    items: [
      {
        type: 'source',
        name: 'Source',
        description: 'External data source (Kafka, HTTP, etc.)',
        icon: ArrowDownToLine,
        color: 'bg-source',
      },
      {
        type: 'trigger',
        name: 'Trigger',
        description: 'Time-based event generator',
        icon: Timer,
        color: 'bg-trigger',
      },
    ],
  },
  {
    id: 'data',
    name: 'Data',
    description: 'Event channels & storage',
    items: [
      {
        type: 'stream',
        name: 'Stream',
        description: 'Event stream - central data channel',
        icon: Database,
        color: 'bg-stream',
      },
      {
        type: 'table',
        name: 'Table',
        description: 'Reference data table with state',
        icon: Table2,
        color: 'bg-table',
      },
    ],
  },
  {
    id: 'filtering',
    name: 'Filtering',
    description: 'Filter and route events',
    items: [
      {
        type: 'filter',
        name: 'Filter',
        description: 'Filter events by condition',
        icon: Filter,
        color: 'bg-filter',
      },
    ],
  },
  {
    id: 'partitioning',
    name: 'Partitioning',
    description: 'Parallel processing',
    items: [
      {
        type: 'partition',
        name: 'Partition',
        description: 'Parallel partitioning by key',
        icon: Layers,
        color: 'bg-partition',
      },
    ],
  },
  {
    id: 'processing',
    name: 'Processing',
    description: 'Transform & aggregate',
    items: [
      {
        type: 'window',
        name: 'Window',
        description: 'Time or count-based windowing',
        icon: Clock,
        color: 'bg-window',
      },
      {
        type: 'aggregation',
        name: 'Aggregation',
        description: 'Aggregate functions (SUM, AVG, etc.)',
        icon: BarChart2,
        color: 'bg-aggregation',
      },
      {
        type: 'groupBy',
        name: 'Group By',
        description: 'Group events by attributes',
        icon: Group,
        color: 'bg-aggregation',
      },
      {
        type: 'join',
        name: 'Join',
        description: 'Join streams or tables',
        icon: GitMerge,
        color: 'bg-join',
      },
      {
        type: 'pattern',
        name: 'Pattern',
        description: 'CEP pattern matching',
        icon: Route,
        color: 'bg-pattern',
      },
    ],
  },
  {
    id: 'selection',
    name: 'Selection',
    description: 'Select output columns',
    items: [
      {
        type: 'projection',
        name: 'Projection',
        description: 'Select and transform columns',
        icon: Columns,
        color: 'bg-projection',
      },
    ],
  },
  {
    id: 'output',
    name: 'Output',
    description: 'Data destinations',
    items: [
      {
        type: 'sink',
        name: 'Sink',
        description: 'External data destination (Kafka, Log, etc.)',
        icon: ArrowUpFromLine,
        color: 'bg-sink',
      },
    ],
  },
];

export function Palette() {
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(
    new Set(['input', 'data', 'filtering', 'partitioning', 'processing', 'selection', 'output'])
  );

  const toggleCategory = (categoryId: string) => {
    setExpandedCategories((prev) => {
      const next = new Set(prev);
      if (next.has(categoryId)) {
        next.delete(categoryId);
      } else {
        next.add(categoryId);
      }
      return next;
    });
  };

  const onDragStart = (event: React.DragEvent, type: ElementType) => {
    event.dataTransfer.setData('application/reactflow', type);
    event.dataTransfer.effectAllowed = 'move';
  };

  return (
    <div className="w-56 border-r border-vscode-border bg-gray-900/30 flex flex-col">
      <div className="px-3 py-2 border-b border-vscode-border">
        <h2 className="text-sm font-medium text-gray-300">Elements</h2>
      </div>

      <div className="flex-1 overflow-y-auto py-2">
        {categories.map((category) => (
          <div key={category.id} className="mb-2">
            <button
              onClick={() => toggleCategory(category.id)}
              className="w-full flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-gray-400 hover:text-gray-200 transition-colors"
            >
              {expandedCategories.has(category.id) ? (
                <ChevronDown className="w-3 h-3" />
              ) : (
                <ChevronRight className="w-3 h-3" />
              )}
              {category.name}
            </button>

            {expandedCategories.has(category.id) && (
              <div className="px-2">
                {category.items.map((item) => (
                  <div
                    key={item.type}
                    draggable
                    onDragStart={(e) => onDragStart(e, item.type)}
                    className="palette-item"
                    title={item.description}
                  >
                    <div className={`palette-item-icon ${item.color}`}>
                      <item.icon className="w-3 h-3" />
                    </div>
                    <span>{item.name}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="px-3 py-2 border-t border-vscode-border text-xs text-gray-500">
        Drag elements to canvas
      </div>
    </div>
  );
}
