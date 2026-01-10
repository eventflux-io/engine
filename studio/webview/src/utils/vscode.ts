// VS Code API interface for webview communication

interface VSCodeAPI {
  postMessage(message: unknown): void;
  getState(): unknown;
  setState(state: unknown): void;
}

declare global {
  interface Window {
    acquireVsCodeApi?: () => VSCodeAPI;
  }
}

// Mock API for development outside VS Code
function createMockApi(): VSCodeAPI {
  console.warn('Running outside VS Code - using mock API');
  return {
    postMessage: (message) => {
      console.log('VS Code postMessage:', message);
    },
    getState: () => {
      const state = localStorage.getItem('vscode-state');
      return state ? JSON.parse(state) : undefined;
    },
    setState: (state) => {
      localStorage.setItem('vscode-state', JSON.stringify(state));
    },
  };
}

// Acquire VS Code API (only available in VS Code webview context)
const vsCodeApi: VSCodeAPI = typeof window.acquireVsCodeApi === 'function'
  ? window.acquireVsCodeApi()
  : createMockApi();

export const vscode = {
  postMessage: (message: unknown) => {
    vsCodeApi.postMessage(message);
  },
  getState: <T>(): T | undefined => {
    return vsCodeApi.getState() as T | undefined;
  },
  setState: <T>(state: T) => {
    vsCodeApi.setState(state);
  },
};
