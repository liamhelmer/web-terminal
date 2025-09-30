/**
 * Terminal wrapper class for xterm.js
 * Provides a high-level interface for terminal operations with custom functionality
 *
 * Per spec-kit 004-frontend-spec.md section 3.1
 */

import { Terminal as XTerm } from 'xterm';
import type { ITerminalOptions } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import { SearchAddon } from 'xterm-addon-search';

/**
 * Terminal configuration interface
 */
export interface TerminalConfig {
  fontSize: number;
  fontFamily: string;
  theme: TerminalTheme;
}

/**
 * Terminal theme interface defining all color properties
 */
export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  selection: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

/**
 * Terminal class wrapping xterm.js with custom functionality
 *
 * Features:
 * - FitAddon for responsive terminal sizing
 * - WebLinksAddon for clickable URLs
 * - SearchAddon for text search functionality
 * - Custom keyboard shortcuts (Ctrl+C, Ctrl+V, Ctrl+F)
 * - Colored output methods (error, info)
 * - Automatic window resize handling
 */
export class Terminal {
  private xterm: XTerm;
  private fitAddon: FitAddon;
  private searchAddon: SearchAddon;
  private container: HTMLElement;
  private resizeHandler: (() => void) | null = null;

  /**
   * Create a new Terminal instance
   * @param container - HTML element to mount the terminal into
   * @param config - Terminal configuration (font, theme, etc.)
   */
  constructor(container: HTMLElement, config: TerminalConfig) {
    this.container = container;

    // Create xterm.js options from config
    const xtermOptions: ITerminalOptions = {
      fontSize: config.fontSize,
      fontFamily: config.fontFamily,
      theme: config.theme,
      cursorBlink: true,
      cursorStyle: 'block',
      scrollback: 10000,
      allowTransparency: false,
      allowProposedApi: true, // Required for some addons
    };

    // Create xterm instance
    this.xterm = new XTerm(xtermOptions);

    // Initialize addons
    this.fitAddon = new FitAddon();
    this.searchAddon = new SearchAddon();

    // Load addons into xterm
    this.xterm.loadAddon(this.fitAddon);
    this.xterm.loadAddon(this.searchAddon);
    this.xterm.loadAddon(new WebLinksAddon());

    // Setup keyboard shortcuts and event handlers
    this.setupEventHandlers();
  }

  /**
   * Open the terminal in the DOM and fit it to the container
   * Sets up automatic resize handling
   */
  open(): void {
    // Mount terminal to DOM
    this.xterm.open(this.container);

    // Initial fit
    this.fit();

    // Setup window resize handler
    this.resizeHandler = () => this.fit();
    window.addEventListener('resize', this.resizeHandler);
  }

  /**
   * Fit the terminal to its container size
   * Should be called when container size changes
   */
  fit(): void {
    try {
      this.fitAddon.fit();
    } catch (error) {
      console.error('Failed to fit terminal:', error);
    }
  }

  /**
   * Write data to the terminal
   * @param data - String to write (can include ANSI escape codes)
   */
  write(data: string): void {
    this.xterm.write(data);
  }

  /**
   * Write an error message to the terminal in red
   * @param message - Error message to display
   */
  writeError(message: string): void {
    // ANSI escape codes: \x1b[31m = red, \x1b[0m = reset
    this.xterm.write(`\x1b[31m${message}\x1b[0m\r\n`);
  }

  /**
   * Write an info message to the terminal in blue
   * @param message - Info message to display
   */
  writeInfo(message: string): void {
    // ANSI escape codes: \x1b[34m = blue, \x1b[0m = reset
    this.xterm.write(`\x1b[34m${message}\x1b[0m\r\n`);
  }

  /**
   * Clear the terminal screen
   */
  clear(): void {
    this.xterm.clear();
  }

  /**
   * Register callback for terminal data input
   * @param callback - Function to call when user types in terminal
   */
  onData(callback: (data: string) => void): void {
    this.xterm.onData(callback);
  }

  /**
   * Register callback for terminal resize events
   * @param callback - Function to call when terminal is resized (cols, rows)
   */
  onResize(callback: (cols: number, rows: number) => void): void {
    this.xterm.onResize(({ cols, rows }) => {
      callback(cols, rows);
    });
  }

  /**
   * Search for text in the terminal
   * @param term - Search term
   * @param forward - Search direction (true = forward, false = backward)
   * @returns true if term was found, false otherwise
   */
  search(term: string, forward: boolean = true): boolean {
    return forward
      ? this.searchAddon.findNext(term)
      : this.searchAddon.findPrevious(term);
  }

  /**
   * Dispose of the terminal and clean up resources
   * Removes event listeners and destroys xterm instance
   */
  dispose(): void {
    // Remove resize handler
    if (this.resizeHandler) {
      window.removeEventListener('resize', this.resizeHandler);
      this.resizeHandler = null;
    }

    // Dispose xterm instance
    this.xterm.dispose();
  }

  /**
   * Setup keyboard shortcuts and custom event handlers
   * @private
   */
  private setupEventHandlers(): void {
    // Attach custom keyboard event handler for shortcuts
    this.xterm.attachCustomKeyEventHandler((event: KeyboardEvent): boolean => {
      // Ctrl+C - Allow default behavior (copy or send SIGINT)
      if (event.ctrlKey && event.key === 'c') {
        return true; // Let xterm handle it
      }

      // Ctrl+V - Paste from clipboard
      if (event.ctrlKey && event.key === 'v') {
        // Prevent default and handle paste
        event.preventDefault();
        this.handlePaste();
        return false;
      }

      // Ctrl+F - Show search dialog
      if (event.ctrlKey && event.key === 'f') {
        event.preventDefault();
        this.showSearchDialog();
        return false;
      }

      // Let xterm handle all other keys
      return true;
    });
  }

  /**
   * Handle paste from clipboard
   * Reads clipboard text and pastes into terminal
   * @private
   */
  private async handlePaste(): Promise<void> {
    try {
      // Read text from clipboard
      const text = await navigator.clipboard.readText();

      // Paste into terminal
      if (text) {
        this.xterm.paste(text);
      }
    } catch (error) {
      console.error('Failed to paste from clipboard:', error);
      this.writeError('Failed to paste: clipboard access denied');
    }
  }

  /**
   * Show search dialog for finding text in terminal
   * Uses browser prompt for simplicity (can be replaced with custom UI)
   * @private
   */
  private showSearchDialog(): void {
    // Simple implementation using browser prompt
    // TODO: Replace with custom search UI component
    const term = prompt('Search in terminal:');

    if (term) {
      const found = this.search(term, true);

      if (!found) {
        this.writeInfo(`No results found for: ${term}`);
      }
    }
  }

  /**
   * Get the underlying xterm.js instance
   * For advanced operations not covered by this wrapper
   * @returns XTerm instance
   */
  getXTermInstance(): XTerm {
    return this.xterm;
  }

  /**
   * Get current terminal dimensions
   * @returns Object with cols and rows
   */
  getDimensions(): { cols: number; rows: number } {
    return {
      cols: this.xterm.cols,
      rows: this.xterm.rows,
    };
  }

  /**
   * Focus the terminal
   * Useful after initialization or when switching between UI elements
   */
  focus(): void {
    this.xterm.focus();
  }

  /**
   * Scroll to the bottom of the terminal
   * Useful for ensuring latest output is visible
   */
  scrollToBottom(): void {
    this.xterm.scrollToBottom();
  }

  /**
   * Check if terminal is currently focused
   * @returns true if terminal has focus
   */
  isFocused(): boolean {
    return this.xterm.textarea?.matches(':focus') ?? false;
  }
}