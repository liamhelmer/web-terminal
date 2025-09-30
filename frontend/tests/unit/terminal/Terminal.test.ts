/**
 * Unit tests for Terminal wrapper class
 * Per spec-kit 008-testing-spec.md
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { Terminal, TerminalConfig, TerminalTheme } from '@/terminal/Terminal';

// Mock xterm.js and addons
vi.mock('xterm', () => ({
  Terminal: vi.fn().mockImplementation(() => ({
    open: vi.fn(),
    write: vi.fn(),
    clear: vi.fn(),
    dispose: vi.fn(),
    onData: vi.fn(),
    onResize: vi.fn(),
    paste: vi.fn(),
    focus: vi.fn(),
    scrollToBottom: vi.fn(),
    loadAddon: vi.fn(),
    attachCustomKeyEventHandler: vi.fn(),
    cols: 80,
    rows: 24,
    textarea: {
      matches: vi.fn().mockReturnValue(false),
    },
  })),
}));

vi.mock('xterm-addon-fit', () => ({
  FitAddon: vi.fn().mockImplementation(() => ({
    fit: vi.fn(),
  })),
}));

vi.mock('xterm-addon-web-links', () => ({
  WebLinksAddon: vi.fn().mockImplementation(() => ({})),
}));

vi.mock('xterm-addon-search', () => ({
  SearchAddon: vi.fn().mockImplementation(() => ({
    findNext: vi.fn().mockReturnValue(true),
    findPrevious: vi.fn().mockReturnValue(true),
  })),
}));

describe('Terminal', () => {
  let container: HTMLElement;
  let config: TerminalConfig;
  let terminal: Terminal;

  const defaultTheme: TerminalTheme = {
    background: '#000000',
    foreground: '#ffffff',
    cursor: '#ffffff',
    selection: '#444444',
    black: '#000000',
    red: '#ff0000',
    green: '#00ff00',
    yellow: '#ffff00',
    blue: '#0000ff',
    magenta: '#ff00ff',
    cyan: '#00ffff',
    white: '#ffffff',
    brightBlack: '#666666',
    brightRed: '#ff6666',
    brightGreen: '#66ff66',
    brightYellow: '#ffff66',
    brightBlue: '#6666ff',
    brightMagenta: '#ff66ff',
    brightCyan: '#66ffff',
    brightWhite: '#ffffff',
  };

  beforeEach(() => {
    // Create container element
    container = document.createElement('div');
    document.body.appendChild(container);

    // Create config
    config = {
      fontSize: 14,
      fontFamily: 'monospace',
      theme: defaultTheme,
    };

    // Create terminal instance
    terminal = new Terminal(container, config);
  });

  afterEach(() => {
    // Clean up
    terminal.dispose();
    document.body.removeChild(container);
  });

  describe('constructor', () => {
    it('should create a Terminal instance', () => {
      expect(terminal).toBeDefined();
      expect(terminal).toBeInstanceOf(Terminal);
    });
  });

  describe('open', () => {
    it('should open terminal in container', () => {
      terminal.open();

      const xterm = terminal.getXTermInstance();
      expect(xterm.open).toHaveBeenCalledWith(container);
    });
  });

  describe('write methods', () => {
    beforeEach(() => {
      terminal.open();
    });

    it('should write data to terminal', () => {
      const data = 'Hello, World!';
      terminal.write(data);

      const xterm = terminal.getXTermInstance();
      expect(xterm.write).toHaveBeenCalledWith(data);
    });

    it('should write error message in red', () => {
      const message = 'Error occurred';
      terminal.writeError(message);

      const xterm = terminal.getXTermInstance();
      expect(xterm.write).toHaveBeenCalledWith(expect.stringContaining(message));
      expect(xterm.write).toHaveBeenCalledWith(expect.stringContaining('\x1b[31m'));
    });

    it('should write info message in blue', () => {
      const message = 'Information';
      terminal.writeInfo(message);

      const xterm = terminal.getXTermInstance();
      expect(xterm.write).toHaveBeenCalledWith(expect.stringContaining(message));
      expect(xterm.write).toHaveBeenCalledWith(expect.stringContaining('\x1b[34m'));
    });
  });

  describe('clear', () => {
    it('should clear terminal screen', () => {
      terminal.open();
      terminal.clear();

      const xterm = terminal.getXTermInstance();
      expect(xterm.clear).toHaveBeenCalled();
    });
  });

  describe('event handlers', () => {
    it('should register onData callback', () => {
      const callback = vi.fn();
      terminal.onData(callback);

      const xterm = terminal.getXTermInstance();
      expect(xterm.onData).toHaveBeenCalledWith(callback);
    });

    it('should register onResize callback', () => {
      const callback = vi.fn();
      terminal.onResize(callback);

      const xterm = terminal.getXTermInstance();
      expect(xterm.onResize).toHaveBeenCalled();
    });
  });

  describe('search', () => {
    beforeEach(() => {
      terminal.open();
    });

    it('should search forward by default', () => {
      const term = 'search term';
      const result = terminal.search(term);

      expect(result).toBe(true);
    });

    it('should search forward when specified', () => {
      const term = 'search term';
      const result = terminal.search(term, true);

      expect(result).toBe(true);
    });

    it('should search backward when specified', () => {
      const term = 'search term';
      const result = terminal.search(term, false);

      expect(result).toBe(true);
    });
  });

  describe('dimensions', () => {
    it('should return current terminal dimensions', () => {
      const dims = terminal.getDimensions();

      expect(dims).toEqual({
        cols: 80,
        rows: 24,
      });
    });
  });

  describe('focus', () => {
    beforeEach(() => {
      terminal.open();
    });

    it('should focus the terminal', () => {
      terminal.focus();

      const xterm = terminal.getXTermInstance();
      expect(xterm.focus).toHaveBeenCalled();
    });

    it('should report focus state', () => {
      const focused = terminal.isFocused();

      expect(typeof focused).toBe('boolean');
    });
  });

  describe('scrollToBottom', () => {
    beforeEach(() => {
      terminal.open();
    });

    it('should scroll terminal to bottom', () => {
      terminal.scrollToBottom();

      const xterm = terminal.getXTermInstance();
      expect(xterm.scrollToBottom).toHaveBeenCalled();
    });
  });

  describe('dispose', () => {
    it('should dispose xterm instance', () => {
      terminal.open();
      terminal.dispose();

      const xterm = terminal.getXTermInstance();
      expect(xterm.dispose).toHaveBeenCalled();
    });

    it('should remove resize event listener', () => {
      terminal.open();

      const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');
      terminal.dispose();

      expect(removeEventListenerSpy).toHaveBeenCalledWith('resize', expect.any(Function));
    });
  });

  describe('getXTermInstance', () => {
    it('should return underlying xterm instance', () => {
      const xterm = terminal.getXTermInstance();

      expect(xterm).toBeDefined();
      expect(xterm).toHaveProperty('write');
      expect(xterm).toHaveProperty('open');
    });
  });
});
