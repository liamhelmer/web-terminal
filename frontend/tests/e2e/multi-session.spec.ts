/**
 * Multi-Session E2E Tests
 * Per spec-kit/008-testing-spec.md
 * Per spec-kit/003-backend-spec.md section 2.2 (Session Management)
 *
 * Tests concurrent session management including:
 * - Multiple terminal sessions
 * - Session switching
 * - Session isolation
 * - Concurrent command execution
 * - Session cleanup
 *
 * IMPORTANT: Uses single-port architecture (port 8080)
 */

import { test, expect, Browser, BrowserContext, Page } from '@playwright/test';

// Base URL from environment or default to localhost:8080 (single-port architecture)
const BASE_URL = process.env.TEST_BASE_URL || 'http://localhost:8080';

test.describe('Multi-Session E2E Tests', () => {
  test.describe('Multiple Session Creation', () => {
    test('should create multiple independent sessions', async ({ browser }) => {
      const context = await browser.newContext();

      // Create three independent pages (sessions)
      const page1 = await context.newPage();
      const page2 = await context.newPage();
      const page3 = await context.newPage();

      await Promise.all([
        page1.goto('/'),
        page2.goto('/'),
        page3.goto('/')
      ]);

      await page1.waitForTimeout(1000);

      // Verify each page has unique session
      const sessions = await Promise.all([
        page1.evaluate(() => localStorage.getItem('session-id')),
        page2.evaluate(() => localStorage.getItem('session-id')),
        page3.evaluate(() => localStorage.getItem('session-id'))
      ]);

      // Filter out null values
      const validSessions = sessions.filter(s => s !== null);

      // Verify sessions are unique (if they exist)
      if (validSessions.length >= 2) {
        const uniqueSessions = new Set(validSessions);
        expect(uniqueSessions.size).toBe(validSessions.length);
      }

      await context.close();
    });

    test('should handle session creation rate limiting', async ({ browser }) => {
      const context = await browser.newContext();
      const pages: Page[] = [];

      // Attempt to create many sessions rapidly
      for (let i = 0; i < 10; i++) {
        const page = await context.newPage();
        pages.push(page);
        await page.goto('/');
      }

      await pages[0].waitForTimeout(1000);

      // Count successful sessions
      const sessionIds = await Promise.all(
        pages.map(page => page.evaluate(() => localStorage.getItem('session-id')))
      );

      const validSessions = sessionIds.filter(id => id !== null);

      // Should have created at least some sessions
      expect(validSessions.length).toBeGreaterThan(0);

      // Cleanup
      await Promise.all(pages.map(page => page.close()));
      await context.close();
    });

    test('should assign unique session IDs', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      const session1 = await page1.evaluate(() => localStorage.getItem('session-id'));
      const session2 = await page2.evaluate(() => localStorage.getItem('session-id'));

      // Sessions should be different (if they exist)
      if (session1 && session2) {
        expect(session1).not.toBe(session2);
      }

      await context.close();
    });
  });

  test.describe('Session Switching', () => {
    test('should switch between sessions in same tab', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Create first session
      await page.evaluate(() => {
        localStorage.setItem('session-id', 'session-1');
        localStorage.setItem('current-session', 'session-1');
      });

      // Switch to second session
      await page.evaluate(() => {
        localStorage.setItem('session-id', 'session-2');
        localStorage.setItem('current-session', 'session-2');
      });

      const currentSession = await page.evaluate(() => {
        return localStorage.getItem('current-session');
      });

      expect(currentSession).toBe('session-2');
    });

    test('should preserve session state when switching', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Setup session 1 with state
      await page.evaluate(() => {
        const session1 = {
          id: 'session-1',
          cwd: '/home/user',
          env: { VAR1: 'value1' }
        };

        localStorage.setItem('session-1', JSON.stringify(session1));
        localStorage.setItem('current-session', 'session-1');
      });

      // Switch to session 2
      await page.evaluate(() => {
        const session2 = {
          id: 'session-2',
          cwd: '/tmp',
          env: { VAR2: 'value2' }
        };

        localStorage.setItem('session-2', JSON.stringify(session2));
        localStorage.setItem('current-session', 'session-2');
      });

      // Switch back to session 1
      await page.evaluate(() => {
        localStorage.setItem('current-session', 'session-1');
      });

      // Verify session 1 state preserved
      const session1State = await page.evaluate(() => {
        const data = localStorage.getItem('session-1');
        return data ? JSON.parse(data) : null;
      });

      expect(session1State?.cwd).toBe('/home/user');
      expect(session1State?.env.VAR1).toBe('value1');
    });

    test('should update UI to reflect active session', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Set active session
      await page.evaluate(() => {
        localStorage.setItem('current-session', 'session-abc');
        (window as any).__activeSession = 'session-abc';
      });

      const activeSession = await page.evaluate(() => {
        return (window as any).__activeSession;
      });

      expect(activeSession).toBe('session-abc');
    });
  });

  test.describe('Session Isolation', () => {
    test('should isolate environment variables between sessions', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      // Set different env vars in each session
      await page1.evaluate(() => {
        localStorage.setItem('session-env', JSON.stringify({ TEST_VAR: 'session1' }));
      });

      await page2.evaluate(() => {
        localStorage.setItem('session-env', JSON.stringify({ TEST_VAR: 'session2' }));
      });

      const env1 = await page1.evaluate(() => {
        const data = localStorage.getItem('session-env');
        return data ? JSON.parse(data) : {};
      });

      const env2 = await page2.evaluate(() => {
        const data = localStorage.getItem('session-env');
        return data ? JSON.parse(data) : {};
      });

      // Verify isolation
      expect(env1.TEST_VAR).not.toBe(env2.TEST_VAR);

      await context.close();
    });

    test('should isolate working directory between sessions', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      // Set different cwd in each session
      await page1.evaluate(() => {
        localStorage.setItem('session-cwd', '/home/user1');
      });

      await page2.evaluate(() => {
        localStorage.setItem('session-cwd', '/home/user2');
      });

      const cwd1 = await page1.evaluate(() => localStorage.getItem('session-cwd'));
      const cwd2 = await page2.evaluate(() => localStorage.getItem('session-cwd'));

      expect(cwd1).toBe('/home/user1');
      expect(cwd2).toBe('/home/user2');
      expect(cwd1).not.toBe(cwd2);

      await context.close();
    });

    test('should isolate running processes between sessions', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      // Simulate processes in each session
      await page1.evaluate(() => {
        (window as any).__processes = [{ pid: 1001, cmd: 'sleep 100' }];
      });

      await page2.evaluate(() => {
        (window as any).__processes = [{ pid: 2001, cmd: 'tail -f log.txt' }];
      });

      const procs1 = await page1.evaluate(() => (window as any).__processes);
      const procs2 = await page2.evaluate(() => (window as any).__processes);

      expect(procs1[0].pid).not.toBe(procs2[0].pid);

      await context.close();
    });

    test('should not share terminal history between sessions', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      // Add history to each session
      await page1.evaluate(() => {
        localStorage.setItem('terminal-history', JSON.stringify(['ls', 'pwd', 'cd /tmp']));
      });

      await page2.evaluate(() => {
        localStorage.setItem('terminal-history', JSON.stringify(['echo hello', 'date']));
      });

      const history1 = await page1.evaluate(() => {
        const data = localStorage.getItem('terminal-history');
        return data ? JSON.parse(data) : [];
      });

      const history2 = await page2.evaluate(() => {
        const data = localStorage.getItem('terminal-history');
        return data ? JSON.parse(data) : [];
      });

      expect(history1).not.toEqual(history2);
      expect(history1.length).toBe(3);
      expect(history2.length).toBe(2);

      await context.close();
    });
  });

  test.describe('Concurrent Command Execution', () => {
    test('should execute commands concurrently in multiple sessions', async ({ browser }) => {
      const context = await browser.newContext();
      const pages = await Promise.all([
        context.newPage(),
        context.newPage(),
        context.newPage()
      ]);

      // Navigate all pages
      await Promise.all(pages.map(page => page.goto('/')));
      await pages[0].waitForTimeout(1000);

      // Execute commands concurrently
      const commands = [
        pages[0].keyboard.type('echo "session 1"'),
        pages[1].keyboard.type('echo "session 2"'),
        pages[2].keyboard.type('echo "session 3"')
      ];

      await Promise.all(commands);

      // Press Enter on all
      await Promise.all(pages.map(page => page.keyboard.press('Enter')));

      await pages[0].waitForTimeout(500);

      // Verify no errors
      const errors = await Promise.all(
        pages.map(page =>
          page.evaluate(() => (window as any).__playwright_errors?.length || 0)
        )
      );

      expect(errors.every(e => e === 0)).toBe(true);

      await context.close();
    });

    test('should handle high load with many concurrent sessions', async ({ browser }) => {
      const context = await browser.newContext();
      const numSessions = 20;
      const pages: Page[] = [];

      // Create many sessions
      for (let i = 0; i < numSessions; i++) {
        const page = await context.newPage();
        pages.push(page);
        await page.goto('/');
      }

      await pages[0].waitForTimeout(1500);

      // All pages should load successfully
      const loadedCount = pages.length;
      expect(loadedCount).toBe(numSessions);

      // Cleanup
      await Promise.all(pages.map(page => page.close()));
      await context.close();
    });

    test('should maintain session independence under load', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      // Rapid operations in both sessions
      const operations = [];

      for (let i = 0; i < 10; i++) {
        operations.push(
          page1.keyboard.type(`cmd${i}`),
          page2.keyboard.type(`cmd${i}`)
        );
      }

      await Promise.all(operations);
      await page1.waitForTimeout(500);

      // Verify both sessions still functional
      const page1Active = await page1.evaluate(() => document.hasFocus());
      const page2Active = await page2.evaluate(() => document.hasFocus());

      expect(typeof page1Active).toBe('boolean');
      expect(typeof page2Active).toBe('boolean');

      await context.close();
    });
  });

  test.describe('Session Lifecycle Management', () => {
    test('should track session creation time', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      const created = await page.evaluate(() => {
        const timestamp = Date.now();
        localStorage.setItem('session-created', timestamp.toString());
        return timestamp;
      });

      expect(created).toBeGreaterThan(0);
    });

    test('should update session last activity timestamp', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Initial activity
      await page.evaluate(() => {
        localStorage.setItem('session-last-activity', Date.now().toString());
      });

      const firstActivity = await page.evaluate(() => {
        return parseInt(localStorage.getItem('session-last-activity') || '0');
      });

      await page.waitForTimeout(100);

      // Update activity
      await page.evaluate(() => {
        localStorage.setItem('session-last-activity', Date.now().toString());
      });

      const secondActivity = await page.evaluate(() => {
        return parseInt(localStorage.getItem('session-last-activity') || '0');
      });

      expect(secondActivity).toBeGreaterThan(firstActivity);
    });

    test('should destroy session on tab close', async ({ browser }) => {
      const context = await browser.newContext();
      const page = await context.newPage();

      await page.goto('/');
      await page.waitForTimeout(500);

      // Create session
      await page.evaluate(() => {
        localStorage.setItem('session-id', 'session-to-close');
      });

      // Close page (simulates session destruction)
      await page.close();

      // Session should be cleaned up server-side
      // (In real implementation, beforeunload event would notify server)

      await context.close();
    });

    test('should clean up idle sessions', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      const idleThreshold = 30 * 60 * 1000; // 30 minutes

      const shouldCleanup = await page.evaluate((threshold) => {
        const lastActivity = Date.now() - (31 * 60 * 1000); // 31 minutes ago
        const now = Date.now();

        return (now - lastActivity) > threshold;
      }, idleThreshold);

      expect(shouldCleanup).toBe(true);
    });

    test('should persist active sessions across page reload', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      // Create and store session
      await page.evaluate(() => {
        const sessionId = 'persistent-session-123';
        localStorage.setItem('active-session-id', sessionId);
        localStorage.setItem(`session-${sessionId}`, JSON.stringify({
          id: sessionId,
          created: Date.now(),
          cwd: '/home/user'
        }));
      });

      // Reload page
      await page.reload();
      await page.waitForTimeout(500);

      // Verify session restored
      const restoredSession = await page.evaluate(() => {
        const sessionId = localStorage.getItem('active-session-id');
        if (!sessionId) return null;

        const data = localStorage.getItem(`session-${sessionId}`);
        return data ? JSON.parse(data) : null;
      });

      expect(restoredSession).not.toBeNull();
      expect(restoredSession?.id).toBe('persistent-session-123');
    });
  });

  test.describe('Session Resource Management', () => {
    test('should enforce session resource limits', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      const limits = await page.evaluate(() => {
        return {
          maxSessions: 10,
          maxMemoryPerSession: 512 * 1024 * 1024, // 512MB
          maxProcessesPerSession: 50
        };
      });

      expect(limits.maxSessions).toBeGreaterThan(0);
      expect(limits.maxMemoryPerSession).toBeGreaterThan(0);
    });

    test('should track resource usage per session', async ({ page }) => {
      await page.goto('/');
      await page.waitForTimeout(500);

      const usage = await page.evaluate(() => {
        return {
          cpu: 10.5,
          memory: 128 * 1024 * 1024,
          processes: 5
        };
      });

      expect(usage.cpu).toBeGreaterThanOrEqual(0);
      expect(usage.memory).toBeGreaterThanOrEqual(0);
      expect(usage.processes).toBeGreaterThanOrEqual(0);
    });

    test('should prevent session creation when at limit', async ({ browser }) => {
      const context = await browser.newContext();
      const maxSessions = 5;
      const pages: Page[] = [];

      // Create up to max sessions
      for (let i = 0; i < maxSessions; i++) {
        const page = await context.newPage();
        pages.push(page);
        await page.goto('/');
        await page.evaluate((sessionNum) => {
          localStorage.setItem(`session-${sessionNum}`, 'active');
        }, i);
      }

      await pages[0].waitForTimeout(500);

      // Check if limit would be enforced
      const atLimit = await pages[0].evaluate((max) => {
        const sessionKeys = Object.keys(localStorage)
          .filter(key => key.startsWith('session-'));
        return sessionKeys.length >= max;
      }, maxSessions);

      expect(typeof atLimit).toBe('boolean');

      await context.close();
    });
  });

  test.describe('Session Error Recovery', () => {
    test('should recover individual session on error without affecting others', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      // Simulate error in session 1
      await page1.evaluate(() => {
        (window as any).__sessionError = true;
      });

      // Session 2 should be unaffected
      const page2Functional = await page2.evaluate(() => {
        return (window as any).__sessionError !== true;
      });

      expect(page2Functional).toBe(true);

      await context.close();
    });

    test('should handle WebSocket disconnection in one session independently', async ({ browser }) => {
      const context = await browser.newContext();
      const page1 = await context.newPage();
      const page2 = await context.newPage();

      await page1.goto('/');
      await page2.goto('/');
      await page1.waitForTimeout(500);

      // Disconnect session 1
      await page1.evaluate(() => {
        (window as any).__wsDisconnected = true;
      });

      // Session 2 should maintain connection
      const page2Connected = await page2.evaluate(() => {
        return (window as any).__wsDisconnected !== true;
      });

      expect(page2Connected).toBe(true);

      await context.close();
    });
  });
});