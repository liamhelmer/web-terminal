/**
 * Mock implementations for xterm.js
 * Per spec-kit/008-testing-spec.md
 */

import { vi } from 'vitest';

// Mock Terminal class
export class Terminal {
  cols = 80;
  rows = 24;
  textarea: HTMLTextAreaElement | null = null;

  private dataHandlers: Set<(data: string) => void> = new Set();
  private resizeHandlers: Set<(dims: { cols: number; rows: number }) => void> = new Set();
  private content = '';

  constructor(public options: unknown = {}) {
    // Create mock textarea for focus testing
    this.textarea = document.createElement('textarea');
  }

  open(container: HTMLElement): void {
    if (this.textarea) {
      container.appendChild(this.textarea);
    }
  }

  write(data: string): void {
    this.content += data;
  }

  clear(): void {
    this.content = '';
  }

  paste(text: string): void {
    this.dataHandlers.forEach((handler) => handler(text));
  }

  onData(callback: (data: string) => void): void {
    this.dataHandlers.add(callback);
  }

  onResize(callback: (dims: { cols: number; rows: number }) => void): void {
    this.resizeHandlers.add(callback);
  }

  attachCustomKeyEventHandler(handler: (event: KeyboardEvent) => boolean): void {
    // Mock implementation
  }

  dispose(): void {
    this.dataHandlers.clear();
    this.resizeHandlers.clear();
    this.textarea = null;
  }

  loadAddon(addon: unknown): void {
    // Mock addon loading
  }

  focus(): void {
    this.textarea?.focus();
  }

  scrollToBottom(): void {
    // Mock scroll
  }

  getContent(): string {
    return this.content;
  }

  simulateResize(cols: number, rows: number): void {
    this.cols = cols;
    this.rows = rows;
    this.resizeHandlers.forEach((handler) => handler({ cols, rows }));
  }
}

// Mock FitAddon
export class FitAddon {
  fit = vi.fn();
}

// Mock WebLinksAddon
export class WebLinksAddon {}

// Mock SearchAddon
export class SearchAddon {
  findNext = vi.fn().mockReturnValue(true);
  findPrevious = vi.fn().mockReturnValue(true);
}
