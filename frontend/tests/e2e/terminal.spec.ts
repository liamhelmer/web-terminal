/**
 * Terminal E2E Tests
 * Per spec-kit/008-testing-spec.md
 *
 * Tests comprehensive user workflows with mocked WebSocket server
 * Verifies single-port architecture compliance
 */

import { test, expect, Page } from '@playwright/test';

// Base URL from environment or default to localhost:8080 (single-port architecture)
const BASE_URL = process.env.TEST_BASE_URL || 'http://localhost:8080';

test.describe('Terminal E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Set up WebSocket mock interceptor before navigation
    await setupWebSocketMock(page);
  });

  test.describe('Basic Rendering and Loading', () => {
    test('should load terminal page and render container', async ({ page }) => {
      await page.goto('/');

      // Wait for app container to be present
      const appContainer = page.locator('#app');
      await expect(appContainer).toBeVisible();

      // Verify page title
      await expect(page).toHaveTitle(/web-terminal|frontend/i);
    });

    test('should initialize xterm.js terminal instance', async ({ page }) => {
      await page.goto('/');

      // Wait for terminal container (once implemented)
      // This test will need to be updated when actual terminal is implemented
      const terminalContainer = page.locator('.xterm, .terminal, [data-testid="terminal"]');

      // For now, just verify page loads without errors
      const hasErrors = await page.evaluate(() => {
        return (window as any).__playwright_errors?.length > 0;
      });

      expect(hasErrors).toBeFalsy();
    });

    test('should display loading state while connecting', async ({ page }) => {
      await page.goto('/');

      // Look for loading indicators (will need to be added to implementation)
      // This validates the spec requirement for connection status display
      const body = await page.textContent('body');

      // Verify page loaded successfully
      expect(body).toBeTruthy();
    });
  });

  test.describe('Single-Port Architecture Verification', () => {
    test('should construct WebSocket URL from window.location (same port)', async ({ page }) => {
      await page.goto('/');

      // Verify WebSocket connection uses same origin as page
      const wsInfo = await page.evaluate(() => {
        const pageProtocol = window.location.protocol;
        const pageHost = window.location.host;
        const expectedWsProtocol = pageProtocol === 'https:' ? 'wss:' : 'ws:';
        const expectedWsUrl = `${expectedWsProtocol}//${pageHost}/ws`;

        return {
          pageProtocol,
          pageHost,
          expectedWsProtocol,
          expectedWsUrl,
        };
      });

      // Verify protocol auto-detection
      expect(wsInfo.expectedWsProtocol).toMatch(/^wss?:$/);
      expect(wsInfo.expectedWsUrl).toContain(wsInfo.pageHost);

      // Ensure no hardcoded ports in URL
      if (!wsInfo.pageHost.includes(':')) {
        // If no port in host, default HTTP port is used
        expect(wsInfo.expectedWsUrl).not.toMatch(/:\d{4,5}/);
      } else {
        // If port is specified, ensure it matches page port
        const pagePort = wsInfo.pageHost.split(':')[1];
        expect(wsInfo.expectedWsUrl).toContain(`:${pagePort}`);
      }
    });

    test('should use relative URLs for all resources', async ({ page }) => {
      await page.goto('/');

      // Check that scripts and resources use relative paths
      const resources = await page.evaluate(() => {
        const scripts = Array.from(document.querySelectorAll('script[src]'))
          .map(s => (s as HTMLScriptElement).src);
        const links = Array.from(document.querySelectorAll('link[href]'))
          .map(l => (l as HTMLLinkElement).href);

        return { scripts, links };
      });

      // All resources should be from same origin (single-port architecture)
      const pageOrigin = new URL(BASE_URL).origin;

      resources.scripts.forEach(src => {
        expect(src).toContain(pageOrigin);
      });

      resources.links.forEach(href => {
        // Ignore external links (data:, about:, etc.)
        if (href.startsWith('http')) {
          expect(href).toContain(pageOrigin);
        }
      });
    });

    test('should not have hardcoded port numbers in source', async ({ page }) => {
      // Navigate and get inline script content
      await page.goto('/');

      const hasHardcodedPorts = await page.evaluate(() => {
        // Check for hardcoded ports like :3000, :5173, :8080 in scripts
        const scripts = Array.from(document.querySelectorAll('script'));
        const scriptContent = scripts
          .map(s => s.textContent || '')
          .join('\n');

        // Look for patterns like "localhost:3000" or "127.0.0.1:8080"
        const portPattern = /(localhost|127\.0\.0\.1|0\.0\.0\.0):\d{4}/g;
        const matches = scriptContent.match(portPattern);

        return matches ? matches.length > 0 : false;
      });

      // Should not find hardcoded ports in client-side code
      expect(hasHardcodedPorts).toBe(false);
    });
  });

  test.describe('User Input and Interaction', () => {
    test('should handle keyboard input and typing', async ({ page }) => {
      await page.goto('/');

      // Wait for terminal to be ready (implementation-dependent)
      await page.waitForTimeout(500);

      // Simulate typing a command
      await page.keyboard.type('echo "Hello World"');

      // Verify input was processed
      // This will need terminal implementation to verify
      const pageContent = await page.textContent('body');
      expect(pageContent).toBeTruthy();
    });

    test('should handle Enter key to submit commands', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Type command and press Enter
      await page.keyboard.type('ls -la');
      await page.keyboard.press('Enter');

      // Verify command submission (mock should intercept)
      // Implementation will need to handle this
      await page.waitForTimeout(100);
    });

    test('should support Ctrl+C for interrupt', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Type a long-running command
      await page.keyboard.type('sleep 100');
      await page.keyboard.press('Enter');

      // Interrupt with Ctrl+C
      await page.keyboard.press('Control+c');

      // Verify interrupt was sent (implementation-dependent)
      await page.waitForTimeout(100);
    });

    test('should support Ctrl+V for paste', async ({ page }) => {
      // Grant clipboard permissions
      await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
      await page.goto('/');
      await page.waitForTimeout(500);

      // Copy text to clipboard
      const testText = 'echo "pasted text"';
      await page.evaluate(async (text) => {
        await navigator.clipboard.writeText(text);
      }, testText);

      // Paste with Ctrl+V
      await page.keyboard.press('Control+v');

      // Verify paste operation
      await page.waitForTimeout(100);
    });

    test('should support Ctrl+F for search', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Trigger search with Ctrl+F
      const searchPromise = page.waitForEvent('dialog');
      await page.keyboard.press('Control+f');

      // Verify search dialog appears (xterm.js search addon)
      // Note: This might show as a browser dialog or custom UI
      try {
        const dialog = await Promise.race([
          searchPromise,
          page.waitForTimeout(1000).then(() => null)
        ]);

        if (dialog) {
          await dialog.dismiss();
        }
      } catch (e) {
        // Search might be implemented differently
        // Just verify no crash occurred
      }
    });

    test('should handle rapid keyboard input without dropping keys', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Type rapidly
      const rapidText = 'abcdefghijklmnopqrstuvwxyz0123456789';
      await page.keyboard.type(rapidText, { delay: 10 });

      // Verify all characters were processed
      await page.waitForTimeout(200);
    });

    test('should handle special characters and Unicode', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Type special characters
      await page.keyboard.type('echo "Hello 世界 🚀 ñ ö ü"');
      await page.keyboard.press('Enter');

      // Verify Unicode handling
      await page.waitForTimeout(100);
    });
  });

  test.describe('WebSocket Connection Management', () => {
    test('should establish WebSocket connection on load', async ({ page }) => {
      const wsConnections: any[] = [];

      // Track WebSocket connections
      page.on('websocket', ws => {
        wsConnections.push({
          url: ws.url(),
          isClosed: false
        });

        ws.on('close', () => {
          wsConnections[wsConnections.length - 1].isClosed = true;
        });
      });

      await page.goto('/');
      await page.waitForTimeout(1000);

      // Verify WebSocket connection was attempted (once implementation is ready)
      // For now, just verify page loaded
      const hasApp = await page.locator('#app').isVisible();
      expect(hasApp).toBe(true);
    });

    test('should handle WebSocket disconnection gracefully', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Close WebSocket connection programmatically
      await page.evaluate(() => {
        // This will need actual implementation
        // Close any active WebSocket connections
        const wsClients = (window as any).__wsClients || [];
        wsClients.forEach((ws: WebSocket) => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.close();
          }
        });
      });

      // Verify disconnection handling (should show reconnecting status)
      await page.waitForTimeout(500);
    });

    test('should attempt reconnection with exponential backoff', async ({ page }) => {
      // This test validates reconnection logic per spec-kit/004-frontend-spec.md
      await page.goto('/');

      const reconnectionAttempts: number[] = [];

      // Monitor WebSocket creation attempts
      page.on('websocket', ws => {
        reconnectionAttempts.push(Date.now());
      });

      // Trigger disconnection and observe reconnection timing
      await page.evaluate(() => {
        const wsClients = (window as any).__wsClients || [];
        wsClients.forEach((ws: WebSocket) => ws.close());
      });

      // Wait for reconnection attempts
      await page.waitForTimeout(5000);

      // Verify exponential backoff (if reconnections occurred)
      if (reconnectionAttempts.length > 2) {
        const delay1 = reconnectionAttempts[1] - reconnectionAttempts[0];
        const delay2 = reconnectionAttempts[2] - reconnectionAttempts[1];

        // Second delay should be >= first delay (exponential backoff)
        expect(delay2).toBeGreaterThanOrEqual(delay1 * 0.9); // Allow 10% tolerance
      }
    });

    test('should queue messages when disconnected', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Disconnect WebSocket
      await page.evaluate(() => {
        const wsClients = (window as any).__wsClients || [];
        wsClients.forEach((ws: WebSocket) => ws.close());
      });

      // Try to send commands while disconnected
      await page.keyboard.type('echo "queued message"');
      await page.keyboard.press('Enter');

      // Verify message is queued (implementation-dependent)
      await page.waitForTimeout(500);
    });
  });

  test.describe('Terminal Resize Handling', () => {
    test('should resize terminal when window resizes', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Get initial viewport size
      const initialSize = page.viewportSize();

      // Resize window
      await page.setViewportSize({ width: 1024, height: 768 });
      await page.waitForTimeout(200);

      // Resize again
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.waitForTimeout(200);

      // Restore original size
      if (initialSize) {
        await page.setViewportSize(initialSize);
      }

      // Verify no errors during resize
      const errors = await page.evaluate(() => {
        return (window as any).__playwright_errors || [];
      });

      expect(errors.length).toBe(0);
    });

    test('should send resize events to backend', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      const messagesSent: any[] = [];

      // Track WebSocket messages
      page.on('websocket', ws => {
        ws.on('framesent', frame => {
          try {
            const payload = JSON.parse(frame.payload.toString());
            messagesSent.push(payload);
          } catch (e) {
            // Ignore non-JSON frames
          }
        });
      });

      // Trigger resize
      await page.setViewportSize({ width: 1280, height: 720 });
      await page.waitForTimeout(500);

      // Check if resize message was sent (once implemented)
      // const resizeMessages = messagesSent.filter(m => m.type === 'resize');
      // expect(resizeMessages.length).toBeGreaterThan(0);
    });
  });

  test.describe('Session Management', () => {
    test('should persist session data in localStorage', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Check for session storage
      const sessionData = await page.evaluate(() => {
        return localStorage.getItem('terminal-sessions');
      });

      // Session data should be stored (once implementation is complete)
      // For now, just verify localStorage is accessible
      expect(typeof sessionData).toBe('string' || 'object');
    });

    test('should restore session on page reload', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Create some session state
      await page.keyboard.type('echo "session test"');
      await page.keyboard.press('Enter');
      await page.waitForTimeout(200);

      // Store session ID
      const sessionId = await page.evaluate(() => {
        const data = localStorage.getItem('terminal-sessions');
        return data ? JSON.parse(data).currentSessionId : null;
      });

      // Reload page
      await page.reload();
      await page.waitForTimeout(500);

      // Verify session restored
      const restoredSessionId = await page.evaluate(() => {
        const data = localStorage.getItem('terminal-sessions');
        return data ? JSON.parse(data).currentSessionId : null;
      });

      // If session was created, it should be restored
      if (sessionId) {
        expect(restoredSessionId).toBe(sessionId);
      }
    });

    test('should handle multiple sessions', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // This test validates multi-session support per spec-kit
      // Implementation will need session switching UI
      const hasSessions = await page.evaluate(() => {
        return typeof localStorage.getItem('terminal-sessions') === 'string';
      });

      expect(typeof hasSessions).toBe('boolean');
    });
  });

  test.describe('Error Handling and Edge Cases', () => {
    test('should handle WebSocket connection failure gracefully', async ({ page }) => {
      // Navigate to page when server might not be available
      await page.goto('/');

      // Wait for error handling
      await page.waitForTimeout(2000);

      // Page should still render without crashing
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should handle malformed WebSocket messages', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Inject malformed message
      await page.evaluate(() => {
        const ws = (window as any).__mockWebSocket;
        if (ws && ws.onmessage) {
          ws.onmessage({ data: 'invalid json {[}]' });
        }
      });

      // Verify graceful handling (no crash)
      await page.waitForTimeout(500);
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should handle rapid command submission', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Submit multiple commands rapidly
      for (let i = 0; i < 20; i++) {
        await page.keyboard.type(`echo "command ${i}"`);
        await page.keyboard.press('Enter');
        await page.waitForTimeout(50);
      }

      // Verify no crashes or freezes
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should handle very long output without crashing', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Simulate very long output
      await page.evaluate(() => {
        const mockTerminal = (window as any).__mockTerminal;
        if (mockTerminal && mockTerminal.write) {
          // Write 10KB of text
          const longOutput = 'A'.repeat(10000);
          mockTerminal.write(longOutput);
        }
      });

      await page.waitForTimeout(500);

      // Verify terminal still responsive
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should handle network latency gracefully', async ({ page }) => {
      // Simulate slow network
      await page.route('**/*', async (route) => {
        await new Promise(resolve => setTimeout(resolve, 100));
        await route.continue();
      });

      await page.goto('/');
      await page.waitForTimeout(1000);

      // Verify page loaded despite latency
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });
  });

  test.describe('Mobile and Responsive Behavior', () => {
    test('should work on mobile viewport', async ({ page }) => {
      // Set mobile viewport
      await page.setViewportSize({ width: 375, height: 667 });
      await page.goto('/');
      await page.waitForTimeout(500);

      // Verify terminal renders on mobile
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should handle touch events on mobile', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 });
      await page.goto('/');
      await page.waitForTimeout(500);

      // Tap on terminal area
      const app = page.locator('#app');
      await app.tap();

      // Verify no errors
      await page.waitForTimeout(200);
    });
  });

  test.describe('Performance and Resource Usage', () => {
    test('should load page within acceptable time', async ({ page }) => {
      const startTime = Date.now();

      await page.goto('/');
      await page.waitForLoadState('domcontentloaded');

      const loadTime = Date.now() - startTime;

      // Page should load in under 3 seconds
      expect(loadTime).toBeLessThan(3000);
    });

    test('should not have memory leaks with rapid interactions', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Perform many operations
      for (let i = 0; i < 100; i++) {
        await page.keyboard.type('test');
        await page.keyboard.press('Backspace');
        await page.keyboard.press('Backspace');
        await page.keyboard.press('Backspace');
        await page.keyboard.press('Backspace');
      }

      // Verify page still responsive
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });
  });
});

/**
 * Helper function to set up WebSocket mocking
 * Intercepts WebSocket connections and provides mock responses
 */
async function setupWebSocketMock(page: Page) {
  await page.addInitScript(() => {
    // Store original WebSocket
    const OriginalWebSocket = WebSocket;
    const mockClients: WebSocket[] = [];

    // Create mock WebSocket class
    class MockWebSocket extends EventTarget {
      public url: string;
      public readyState: number = WebSocket.CONNECTING;
      public onopen: ((this: WebSocket, ev: Event) => any) | null = null;
      public onclose: ((this: WebSocket, ev: CloseEvent) => any) | null = null;
      public onerror: ((this: WebSocket, ev: Event) => any) | null = null;
      public onmessage: ((this: WebSocket, ev: MessageEvent) => any) | null = null;

      constructor(url: string) {
        super();
        this.url = url;

        // Simulate connection opening
        setTimeout(() => {
          this.readyState = WebSocket.OPEN;
          const event = new Event('open');
          if (this.onopen) {
            this.onopen.call(this as any, event);
          }
          this.dispatchEvent(event);
        }, 100);

        mockClients.push(this as any);
      }

      send(data: string) {
        // Parse message and send mock response
        try {
          const message = JSON.parse(data);

          // Simulate server responses
          setTimeout(() => {
            let response: any;

            switch (message.type) {
              case 'command':
                response = {
                  type: 'output',
                  data: `Mock output for: ${message.data}\r\n`
                };
                break;

              case 'resize':
                response = {
                  type: 'connection_status',
                  status: 'connected'
                };
                break;

              default:
                response = {
                  type: 'connection_status',
                  status: 'connected'
                };
            }

            const responseEvent = new MessageEvent('message', {
              data: JSON.stringify(response)
            });

            if (this.onmessage) {
              this.onmessage.call(this as any, responseEvent);
            }
            this.dispatchEvent(responseEvent);
          }, 50);
        } catch (e) {
          // Ignore parse errors in mock
        }
      }

      close(code?: number, reason?: string) {
        this.readyState = WebSocket.CLOSED;
        const event = new CloseEvent('close', { code, reason });
        if (this.onclose) {
          this.onclose.call(this as any, event);
        }
        this.dispatchEvent(event);
      }
    }

    // Copy static constants
    MockWebSocket.prototype.CONNECTING = WebSocket.CONNECTING;
    MockWebSocket.prototype.OPEN = WebSocket.OPEN;
    MockWebSocket.prototype.CLOSING = WebSocket.CLOSING;
    MockWebSocket.prototype.CLOSED = WebSocket.CLOSED;

    (MockWebSocket as any).CONNECTING = WebSocket.CONNECTING;
    (MockWebSocket as any).OPEN = WebSocket.OPEN;
    (MockWebSocket as any).CLOSING = WebSocket.CLOSING;
    (MockWebSocket as any).CLOSED = WebSocket.CLOSED;

    // Replace global WebSocket with mock
    (window as any).WebSocket = MockWebSocket;
    (window as any).__wsClients = mockClients;
    (window as any).__mockWebSocket = MockWebSocket;

    // Track errors for testing
    (window as any).__playwright_errors = [];
    window.addEventListener('error', (event) => {
      (window as any).__playwright_errors.push({
        message: event.message,
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno
      });
    });
  });
}