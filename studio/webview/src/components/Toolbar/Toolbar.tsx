import { Play, Save, FileCode, Settings, Undo, Redo, Layout, Maximize2 } from 'lucide-react';
import { useApplicationStore } from '../../stores/applicationStore';
import { vscode } from '../../utils/vscode';

export function Toolbar() {
  const { name, isDirty, save, generatedSQL, viewMode, setViewMode } = useApplicationStore();

  const handleSave = () => {
    save();
  };

  const handleShowSQL = () => {
    vscode.postMessage({ type: 'showSQL', sql: generatedSQL });
  };

  const handleSaveSQL = () => {
    vscode.postMessage({ type: 'saveSQL', sql: generatedSQL });
  };

  const handleRun = () => {
    vscode.postMessage({ type: 'runSimulation', application: {} });
  };

  const handleAutoLayout = () => {
    // TODO: Implement auto-layout with dagre
  };

  return (
    <div className="h-12 border-b border-vscode-border flex items-center px-4 gap-2 bg-gray-900/50">
      {/* App name */}
      <div className="flex items-center gap-2 mr-4">
        <span className="font-medium">{name}</span>
        {isDirty && <span className="text-xs text-gray-500">(unsaved)</span>}
      </div>

      <div className="h-6 w-px bg-vscode-border mx-2" />

      {/* File actions */}
      <ToolbarButton icon={Save} label="Save" onClick={handleSave} shortcut="Ctrl+S" />
      <ToolbarButton icon={FileCode} label="View SQL" onClick={handleShowSQL} />
      <ToolbarButton icon={FileCode} label="Export SQL" onClick={handleSaveSQL} />

      <div className="h-6 w-px bg-vscode-border mx-2" />

      {/* Edit actions */}
      <ToolbarButton icon={Undo} label="Undo" onClick={() => {}} shortcut="Ctrl+Z" disabled />
      <ToolbarButton icon={Redo} label="Redo" onClick={() => {}} shortcut="Ctrl+Y" disabled />

      <div className="h-6 w-px bg-vscode-border mx-2" />

      {/* View actions */}
      <ToolbarButton icon={Layout} label="Auto Layout" onClick={handleAutoLayout} />
      <ToolbarButton icon={Maximize2} label="Fit View" onClick={() => {}} />

      <div className="flex-1" />

      {/* View mode toggle */}
      <div className="flex items-center gap-1 bg-gray-800 rounded p-1">
        <ViewModeButton
          active={viewMode === 'visual'}
          onClick={() => setViewMode('visual')}
          label="Visual"
        />
        <ViewModeButton
          active={viewMode === 'split'}
          onClick={() => setViewMode('split')}
          label="Split"
        />
        <ViewModeButton
          active={viewMode === 'sql'}
          onClick={() => setViewMode('sql')}
          label="SQL"
        />
      </div>

      <div className="h-6 w-px bg-vscode-border mx-2" />

      {/* Run */}
      <button
        onClick={handleRun}
        className="flex items-center gap-2 px-3 py-1.5 bg-green-600 hover:bg-green-500 text-white rounded text-sm font-medium transition-colors"
      >
        <Play className="w-4 h-4" />
        Run
      </button>

      <ToolbarButton icon={Settings} label="Settings" onClick={() => {}} />
    </div>
  );
}

interface ToolbarButtonProps {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  onClick: () => void;
  shortcut?: string;
  disabled?: boolean;
}

function ToolbarButton({ icon: Icon, label, onClick, shortcut, disabled }: ToolbarButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`
        flex items-center gap-1.5 px-2 py-1.5 rounded text-sm
        transition-colors
        ${disabled
          ? 'text-gray-600 cursor-not-allowed'
          : 'text-gray-300 hover:bg-vscode-list-hover hover:text-white'
        }
      `}
      title={shortcut ? `${label} (${shortcut})` : label}
    >
      <Icon className="w-4 h-4" />
      <span className="hidden lg:inline">{label}</span>
    </button>
  );
}

interface ViewModeButtonProps {
  active: boolean;
  onClick: () => void;
  label: string;
}

function ViewModeButton({ active, onClick, label }: ViewModeButtonProps) {
  return (
    <button
      onClick={onClick}
      className={`
        px-3 py-1 rounded text-xs font-medium transition-colors
        ${active
          ? 'bg-vscode-button-bg text-white'
          : 'text-gray-400 hover:text-white'
        }
      `}
    >
      {label}
    </button>
  );
}
