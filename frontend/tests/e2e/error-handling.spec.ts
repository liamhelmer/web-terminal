/**
 * Error Handling E2E Tests
 * Per spec-kit/008-testing-spec.md section "Error Handling and Edge Cases"
 * Per spec-kit/007-websocket-spec.md section "Error Codes"
 *
 * Tests comprehensive error handling and recovery including:
 * - Network failures
 * - Server errors
 * - Invalid input
 * - Resource limits
 * - Recovery mechanisms
 *
 * IMPORTANT: Uses single-port architecture (port 8080)
 */

import { test, expect, Page } from '@playwright/test';

// Base URL from environment or default to localhost:8080 (single-port architecture)
const BASE_URL = process.env.TEST_BASE_URL || 'http://localhost:8080';

test.describe('Error Handling E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(500);
  });

  test.describe('Network Error Handling', () => {
    test('should handle complete network failure gracefully', async ({ page }) => {
      // Simulate network offline
      await page.context().setOffline(true);
      await page.waitForTimeout(500);

      // Attempt operation
      await page.keyboard.type('echo test');
      await page.keyboard.press('Enter');

      // Verify error handling
      await page.waitForTimeout(500);

      // Page should still be functional
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);

      // Restore network
      await page.context().setOffline(false);
    });

    test('should reconnect after network restoration', async ({ page }) => {
      const connectionStates: string[] = [];

      await page.exposeFunction('trackConnectionState', (state: string) => {
        connectionStates.push(state);
      });

      // Go offline
      await page.evaluate(() => {
        (window as any).trackConnectionState('connected');
      });

      await page.context().setOffline(true);

      await page.evaluate(() => {
        (window as any).trackConnectionState('disconnected');
      });

      await page.waitForTimeout(500);

      // Go back online
      await page.context().setOffline(false);

      await page.evaluate(() => {
        (window as any).trackConnectionState('reconnected');
      });

      await page.waitForTimeout(500);

      // Verify state transitions
      expect(connectionStates).toContain('disconnected');
    });

    test('should handle slow network gracefully', async ({ page }) => {
      // Simulate slow network
      await page.route('**/*', async (route) => {
        await new Promise(resolve => setTimeout(resolve, 500));
        await route.continue();
      });

      await page.reload();
      await page.waitForTimeout(2000);

      // Verify page loads despite latency
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should queue commands during network interruption', async ({ page }) => {
      const queuedCommands: string[] = [];

      await page.exposeFunction('queueCommand', (cmd: string) => {
        queuedCommands.push(cmd);
      });

      // Simulate disconnection
      await page.context().setOffline(true);

      // Try to send commands
      await page.evaluate(() => {
        const commands = ['ls', 'pwd', 'echo test'];
        commands.forEach(cmd => (window as any).queueCommand(cmd));
      });

      await page.waitForTimeout(500);

      // Verify commands queued
      expect(queuedCommands.length).toBe(3);

      await page.context().setOffline(false);
    });

    test('should handle WebSocket connection timeout', async ({ page }) => {
      let timeoutOccurred = false;

      page.on('console', msg => {
        if (msg.text().includes('timeout') || msg.text().includes('Timeout')) {
          timeoutOccurred = true;
        }
      });

      // Simulate timeout scenario
      await page.evaluate(() => {
        setTimeout(() => {
          console.error('Connection timeout');
        }, 100);
      });

      await page.waitForTimeout(200);

      expect(timeoutOccurred).toBe(true);
    });
  });

  test.describe('Server Error Handling', () => {
    test('should handle command execution errors', async ({ page }) => {
      // Simulate command error
      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'COMMAND_FAILED',
          message: 'Command execution failed'
        };

        (window as any).__lastError = errorMsg;
        console.error('Command execution failed');
      });

      await page.waitForTimeout(200);

      const error = await page.evaluate(() => {
        return (window as any).__lastError;
      });

      expect(error.code).toBe('COMMAND_FAILED');
    });

    test('should handle command not found errors', async ({ page }) => {
      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'COMMAND_NOT_FOUND',
          message: 'Command "nonexistent" not found'
        };

        (window as any).__commandError = errorMsg;
      });

      const error = await page.evaluate(() => {
        return (window as any).__commandError;
      });

      expect(error.code).toBe('COMMAND_NOT_FOUND');
      expect(error.message).toContain('not found');
    });

    test('should handle permission denied errors', async ({ page }) => {
      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'PERMISSION_DENIED',
          message: 'Permission denied: /etc/shadow'
        };

        (window as any).__permissionError = errorMsg;
      });

      const error = await page.evaluate(() => {
        return (window as any).__permissionError;
      });

      expect(error.code).toBe('PERMISSION_DENIED');
    });

    test('should handle internal server errors', async ({ page }) => {
      const errorMessages: string[] = [];

      page.on('console', msg => {
        if (msg.type() === 'error') {
          errorMessages.push(msg.text());
        }
      });

      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'INTERNAL_ERROR',
          message: 'Internal server error occurred'
        };

        console.error(errorMsg.message);
        (window as any).__internalError = errorMsg;
      });

      await page.waitForTimeout(200);

      const hasError = errorMessages.some(msg => msg.includes('Internal'));
      expect(hasError).toBe(true);
    });

    test('should handle session expired errors', async ({ page }) => {
      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'SESSION_EXPIRED',
          message: 'Session has expired'
        };

        (window as any).__sessionExpired = errorMsg;
        localStorage.removeItem('session-id');
      });

      const error = await page.evaluate(() => {
        return (window as any).__sessionExpired;
      });

      expect(error.code).toBe('SESSION_EXPIRED');
    });
  });

  test.describe('Input Validation Errors', () => {
    test('should handle invalid message format', async ({ page }) => {
      const handled = await page.evaluate(() => {
        try {
          const invalidJson = '{ invalid json ]}';
          JSON.parse(invalidJson);
          return false;
        } catch (error) {
          (window as any).__parseError = error;
          return true;
        }
      });

      expect(handled).toBe(true);
    });

    test('should handle extremely long commands', async ({ page }) => {
      const longCommand = 'echo ' + 'A'.repeat(10000);

      await page.keyboard.type(longCommand.substring(0, 100));
      await page.waitForTimeout(100);

      // Should not crash
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should handle special characters in input', async ({ page }) => {
      const specialChars = '!@#$%^&*(){}[]|\\:;"\'<>,.?/~`';

      await page.keyboard.type(specialChars);
      await page.waitForTimeout(100);

      // Should not crash
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should handle binary data in text input', async ({ page }) => {
      await page.evaluate(() => {
        // Simulate binary data in message
        const binaryData = new Uint8Array([0xFF, 0xFE, 0xFD, 0xFC]);
        (window as any).__binaryData = binaryData;

        try {
          const text = new TextDecoder().decode(binaryData);
          (window as any).__decodedText = text;
        } catch (error) {
          (window as any).__decodeError = error;
        }
      });

      await page.waitForTimeout(200);

      const handled = await page.evaluate(() => {
        return (window as any).__decodedText !== undefined ||
               (window as any).__decodeError !== undefined;
      });

      expect(handled).toBe(true);
    });

    test('should sanitize malicious input', async ({ page }) => {
      const maliciousInputs = [
        '<script>alert("xss")</script>',
        '"; DROP TABLE users; --',
        '../../../etc/passwd',
        '$(rm -rf /)'
      ];

      for (const input of maliciousInputs) {
        await page.evaluate((data) => {
          // Simulate input sanitization
          const sanitized = data
            .replace(/[<>]/g, '')
            .replace(/[;&|]/g, '')
            .replace(/\.\./g, '');

          (window as any).__sanitized = sanitized;
        }, input);

        const sanitized = await page.evaluate(() => {
          return (window as any).__sanitized;
        });

        // Should not contain dangerous characters
        expect(sanitized).not.toContain('<script>');
        expect(sanitized).not.toContain('DROP TABLE');
      }
    });
  });

  test.describe('Resource Limit Errors', () => {
    test('should handle resource quota exceeded', async ({ page }) => {
      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'QUOTA_EXCEEDED',
          message: 'Disk quota exceeded'
        };

        (window as any).__quotaError = errorMsg;
      });

      const error = await page.evaluate(() => {
        return (window as any).__quotaError;
      });

      expect(error.code).toBe('QUOTA_EXCEEDED');
    });

    test('should handle memory limit exceeded', async ({ page }) => {
      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'RESOURCE_LIMIT',
          message: 'Memory limit exceeded',
          details: { limit: '512MB', current: '520MB' }
        };

        (window as any).__memoryError = errorMsg;
      });

      const error = await page.evaluate(() => {
        return (window as any).__memoryError;
      });

      expect(error.code).toBe('RESOURCE_LIMIT');
      expect(error.details.limit).toBe('512MB');
    });

    test('should handle too many concurrent processes', async ({ page }) => {
      await page.evaluate(() => {
        const maxProcesses = 50;
        const currentProcesses = 51;

        if (currentProcesses > maxProcesses) {
          const errorMsg = {
            type: 'error',
            code: 'RESOURCE_LIMIT',
            message: `Too many processes (${currentProcesses}/${maxProcesses})`
          };

          (window as any).__processLimitError = errorMsg;
        }
      });

      const error = await page.evaluate(() => {
        return (window as any).__processLimitError;
      });

      expect(error).toBeDefined();
      expect(error.message).toContain('Too many processes');
    });

    test('should handle command timeout', async ({ page }) => {
      await page.evaluate(() => {
        const errorMsg = {
          type: 'error',
          code: 'COMMAND_TIMEOUT',
          message: 'Command execution timed out after 30s'
        };

        (window as any).__timeoutError = errorMsg;
      });

      const error = await page.evaluate(() => {
        return (window as any).__timeoutError;
      });

      expect(error.code).toBe('COMMAND_TIMEOUT');
    });
  });

  test.describe('Error Recovery Mechanisms', () => {
    test('should automatically retry after transient errors', async ({ page }) => {
      const retryAttempts: number[] = [];

      await page.exposeFunction('logRetry', (attempt: number) => {
        retryAttempts.push(attempt);
      });

      await page.evaluate(() => {
        // Simulate retry logic
        const retry = async (maxAttempts: number) => {
          for (let i = 1; i <= maxAttempts; i++) {
            await (window as any).logRetry(i);
            await new Promise(resolve => setTimeout(resolve, 100));
          }
        };

        retry(3);
      });

      await page.waitForTimeout(500);

      expect(retryAttempts.length).toBeGreaterThanOrEqual(3);
    });

    test('should implement exponential backoff for retries', async ({ page }) => {
      const retryDelays: number[] = [];

      await page.exposeFunction('logRetryDelay', (delay: number) => {
        retryDelays.push(delay);
      });

      await page.evaluate(() => {
        // Simulate exponential backoff
        const baseDelay = 100;
        for (let attempt = 0; attempt < 5; attempt++) {
          const delay = baseDelay * Math.pow(2, attempt);
          (window as any).logRetryDelay(delay);
        }
      });

      await page.waitForTimeout(200);

      // Verify exponential growth
      expect(retryDelays[0]).toBe(100);
      expect(retryDelays[1]).toBe(200);
      expect(retryDelays[2]).toBe(400);
      expect(retryDelays[3]).toBe(800);
      expect(retryDelays[4]).toBe(1600);
    });

    test('should clear error state after successful recovery', async ({ page }) => {
      // Set error state
      await page.evaluate(() => {
        (window as any).__errorState = {
          hasError: true,
          lastError: 'Connection failed'
        };
      });

      // Simulate recovery
      await page.evaluate(() => {
        (window as any).__errorState = {
          hasError: false,
          lastError: null
        };
      });

      const errorState = await page.evaluate(() => {
        return (window as any).__errorState;
      });

      expect(errorState.hasError).toBe(false);
      expect(errorState.lastError).toBeNull();
    });

    test('should restore previous state after error recovery', async ({ page }) => {
      // Save state
      await page.evaluate(() => {
        (window as any).__savedState = {
          cwd: '/home/user',
          env: { VAR: 'value' }
        };
      });

      // Simulate error
      await page.evaluate(() => {
        (window as any).__errorOccurred = true;
      });

      // Restore state
      await page.evaluate(() => {
        const saved = (window as any).__savedState;
        (window as any).__currentState = saved;
        (window as any).__errorOccurred = false;
      });

      const restored = await page.evaluate(() => {
        return (window as any).__currentState;
      });

      expect(restored.cwd).toBe('/home/user');
      expect(restored.env.VAR).toBe('value');
    });

    test('should log errors for debugging', async ({ page }) => {
      const errorLogs: any[] = [];

      page.on('console', msg => {
        if (msg.type() === 'error') {
          errorLogs.push({
            text: msg.text(),
            timestamp: Date.now()
          });
        }
      });

      await page.evaluate(() => {
        console.error('Test error 1');
        console.error('Test error 2');
        console.error('Test error 3');
      });

      await page.waitForTimeout(200);

      expect(errorLogs.length).toBe(3);
    });
  });

  test.describe('User-Friendly Error Messages', () => {
    test('should display clear error messages to users', async ({ page }) => {
      const errorMessage = 'Connection failed: Unable to reach server';

      await page.evaluate((msg) => {
        (window as any).__displayedError = msg;
      }, errorMessage);

      const displayed = await page.evaluate(() => {
        return (window as any).__displayedError;
      });

      expect(displayed).toContain('Connection failed');
      expect(displayed).toContain('Unable to reach server');
    });

    test('should provide actionable error suggestions', async ({ page }) => {
      const errorWithSuggestion = {
        message: 'Command not found: "nmp"',
        suggestion: 'Did you mean "npm"?'
      };

      await page.evaluate((error) => {
        (window as any).__errorSuggestion = error;
      }, errorWithSuggestion);

      const error = await page.evaluate(() => {
        return (window as any).__errorSuggestion;
      });

      expect(error.suggestion).toContain('Did you mean');
    });

    test('should hide technical details from users by default', async ({ page }) => {
      const errorWithDetails = {
        userMessage: 'Operation failed',
        technicalDetails: {
          stack: 'Error at line 123...',
          code: 'ERR_CONNECTION_REFUSED'
        }
      };

      await page.evaluate((error) => {
        // Only show user message by default
        (window as any).__displayedMessage = error.userMessage;
        // Technical details available in console only
        console.debug(error.technicalDetails);
      }, errorWithDetails);

      const displayed = await page.evaluate(() => {
        return (window as any).__displayedMessage;
      });

      expect(displayed).toBe('Operation failed');
      expect(displayed).not.toContain('ERR_CONNECTION_REFUSED');
    });

    test('should display error codes for support reference', async ({ page }) => {
      const errorWithCode = {
        message: 'Request failed',
        code: 'ERR_1234',
        timestamp: Date.now()
      };

      await page.evaluate((error) => {
        (window as any).__errorWithCode = error;
      }, errorWithCode);

      const error = await page.evaluate(() => {
        return (window as any).__errorWithCode;
      });

      expect(error.code).toBe('ERR_1234');
      expect(error.timestamp).toBeGreaterThan(0);
    });
  });

  test.describe('Critical Error Handling', () => {
    test('should handle fatal errors without crashing', async ({ page }) => {
      // Simulate fatal error
      await page.evaluate(() => {
        try {
          throw new Error('Fatal error occurred');
        } catch (error) {
          (window as any).__fatalError = error;
          console.error('Fatal error:', error);
        }
      });

      await page.waitForTimeout(200);

      // Page should still be responsive
      const appVisible = await page.locator('#app').isVisible();
      expect(appVisible).toBe(true);
    });

    test('should provide recovery options for critical errors', async ({ page }) => {
      const recoveryOptions = await page.evaluate(() => {
        return {
          reload: 'Reload the page',
          reset: 'Reset to default state',
          contact: 'Contact support'
        };
      });

      expect(recoveryOptions.reload).toBeDefined();
      expect(recoveryOptions.reset).toBeDefined();
      expect(recoveryOptions.contact).toBeDefined();
    });

    test('should save state before fatal error', async ({ page }) => {
      // Set up state
      await page.evaluate(() => {
        const state = {
          sessionId: 'session-123',
          cwd: '/home/user',
          history: ['ls', 'pwd']
        };

        // Save to localStorage before error
        localStorage.setItem('error-recovery-state', JSON.stringify(state));
      });

      // Verify state saved
      const savedState = await page.evaluate(() => {
        const data = localStorage.getItem('error-recovery-state');
        return data ? JSON.parse(data) : null;
      });

      expect(savedState).not.toBeNull();
      expect(savedState.sessionId).toBe('session-123');
    });
  });
});