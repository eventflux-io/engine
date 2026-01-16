/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // VS Code theme colors
        'vscode-bg': 'var(--vscode-editor-background)',
        'vscode-fg': 'var(--vscode-editor-foreground)',
        'vscode-border': 'var(--vscode-panel-border)',
        'vscode-input-bg': 'var(--vscode-input-background)',
        'vscode-input-fg': 'var(--vscode-input-foreground)',
        'vscode-button-bg': 'var(--vscode-button-background)',
        'vscode-button-fg': 'var(--vscode-button-foreground)',
        'vscode-button-hover': 'var(--vscode-button-hoverBackground)',
        'vscode-list-hover': 'var(--vscode-list-hoverBackground)',
        'vscode-list-active': 'var(--vscode-list-activeSelectionBackground)',
        'vscode-focus-border': 'var(--vscode-focusBorder)',
        // Element colors
        'stream': '#3b82f6',      // blue-500
        'table': '#8b5cf6',       // violet-500
        'trigger': '#f59e0b',     // amber-500
        'window': '#10b981',      // emerald-500
        'filter': '#ef4444',      // red-500
        'projection': '#6366f1',  // indigo-500
        'aggregation': '#14b8a6', // teal-500
        'join': '#f97316',        // orange-500
        'pattern': '#ec4899',     // pink-500
        'partition': '#84cc16',   // lime-500
        'output': '#06b6d4',      // cyan-500
        // Connector colors (external systems)
        'source': '#22c55e',      // green-500 - data coming IN
        'sink': '#a855f7',        // purple-500 - data going OUT
      },
      fontFamily: {
        mono: ['var(--vscode-editor-font-family)', 'monospace'],
      },
      fontSize: {
        'vscode': 'var(--vscode-editor-font-size)',
      },
    },
  },
  plugins: [],
};
