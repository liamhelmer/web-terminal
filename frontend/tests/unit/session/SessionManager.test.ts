/**
 * Unit tests for SessionManager
 * Per spec-kit/008-testing-spec.md and 004-frontend-spec.md section 3.3
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';

// SessionManager implementation for testing (based on spec)
interface Session {
  id: string;
  createdAt: Date;
  state: SessionState;
}

interface SessionState {
  workingDir: string;
  environment: Record<string, string>;
  history: string[];
}

class SessionManager {
  private sessions: Map<string, Session> = new Map();
  private currentSession: Session | null = null;

  async createSession(): Promise<Session> {
    const session: Session = {
      id: this.generateSessionId(),
      createdAt: new Date(),
      state: {
        workingDir: '/workspace',
        environment: {},
        history: [],
      },
    };

    this.sessions.set(session.id, session);
    this.currentSession = session;

    // Save to local storage
    this.saveToStorage();

    return session;
  }

  getSession(id: string): Session | undefined {
    return this.sessions.get(id);
  }

  getCurrentSession(): Session | null {
    return this.currentSession;
  }

  switchSession(id: string): boolean {
    const session = this.sessions.get(id);
    if (session) {
      this.currentSession = session;
      this.saveToStorage();
      return true;
    }
    return false;
  }

  deleteSession(id: string): void {
    this.sessions.delete(id);
    if (this.currentSession?.id === id) {
      this.currentSession = null;
    }
    this.saveToStorage();
  }

  listSessions(): Session[] {
    return Array.from(this.sessions.values());
  }

  private generateSessionId(): string {
    return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
  }

  private saveToStorage(): void {
    const data = {
      sessions: Array.from(this.sessions.entries()),
      currentSessionId: this.currentSession?.id,
    };
    localStorage.setItem('terminal-sessions', JSON.stringify(data));
  }

  loadFromStorage(): void {
    const data = localStorage.getItem('terminal-sessions');
    if (data) {
      try {
        const parsed = JSON.parse(data);

        // Restore sessions with Date objects
        const sessions = parsed.sessions.map(([id, session]: [string, any]) => [
          id,
          {
            ...session,
            createdAt: new Date(session.createdAt),
          },
        ]);

        this.sessions = new Map(sessions);

        if (parsed.currentSessionId) {
          this.currentSession = this.sessions.get(parsed.currentSessionId) ?? null;
        }
      } catch (error) {
        console.error('Failed to load sessions from storage:', error);
      }
    }
  }

  clearAllSessions(): void {
    this.sessions.clear();
    this.currentSession = null;
    localStorage.removeItem('terminal-sessions');
  }
}

describe('SessionManager', () => {
  let manager: SessionManager;

  beforeEach(() => {
    manager = new SessionManager();
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  describe('Session Creation', () => {
    it('should create a new session', async () => {
      const session = await manager.createSession();

      expect(session).toBeDefined();
      expect(session.id).toBeTruthy();
      expect(session.createdAt).toBeInstanceOf(Date);
      expect(session.state).toBeDefined();
    });

    it('should generate unique session IDs', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      expect(session1.id).not.toBe(session2.id);
    });

    it('should initialize session with default state', async () => {
      const session = await manager.createSession();

      expect(session.state.workingDir).toBe('/workspace');
      expect(session.state.environment).toEqual({});
      expect(session.state.history).toEqual([]);
    });

    it('should set created session as current', async () => {
      const session = await manager.createSession();

      expect(manager.getCurrentSession()).toBe(session);
    });

    it('should save session to localStorage', async () => {
      const session = await manager.createSession();

      const stored = localStorage.getItem('terminal-sessions');
      expect(stored).toBeTruthy();

      const parsed = JSON.parse(stored!);
      expect(parsed.sessions).toHaveLength(1);
      expect(parsed.sessions[0][0]).toBe(session.id);
    });
  });

  describe('Session Retrieval', () => {
    it('should retrieve session by ID', async () => {
      const session = await manager.createSession();

      const retrieved = manager.getSession(session.id);

      expect(retrieved).toBe(session);
    });

    it('should return undefined for non-existent session', () => {
      const retrieved = manager.getSession('non-existent-id');

      expect(retrieved).toBeUndefined();
    });

    it('should get current session', async () => {
      const session = await manager.createSession();

      const current = manager.getCurrentSession();

      expect(current).toBe(session);
    });

    it('should return null when no current session', () => {
      const current = manager.getCurrentSession();

      expect(current).toBeNull();
    });
  });

  describe('Session Switching', () => {
    it('should switch to existing session', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      const switched = manager.switchSession(session1.id);

      expect(switched).toBe(true);
      expect(manager.getCurrentSession()).toBe(session1);
    });

    it('should return false for non-existent session', async () => {
      await manager.createSession();

      const switched = manager.switchSession('non-existent-id');

      expect(switched).toBe(false);
    });

    it('should save current session to localStorage on switch', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      manager.switchSession(session1.id);

      const stored = localStorage.getItem('terminal-sessions');
      const parsed = JSON.parse(stored!);

      expect(parsed.currentSessionId).toBe(session1.id);
    });
  });

  describe('Session Deletion', () => {
    it('should delete session by ID', async () => {
      const session = await manager.createSession();

      manager.deleteSession(session.id);

      expect(manager.getSession(session.id)).toBeUndefined();
    });

    it('should clear current session if deleting current', async () => {
      const session = await manager.createSession();

      manager.deleteSession(session.id);

      expect(manager.getCurrentSession()).toBeNull();
    });

    it('should not clear current session if deleting non-current', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      manager.deleteSession(session1.id);

      expect(manager.getCurrentSession()).toBe(session2);
    });

    it('should update localStorage after deletion', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      manager.deleteSession(session1.id);

      const stored = localStorage.getItem('terminal-sessions');
      const parsed = JSON.parse(stored!);

      expect(parsed.sessions).toHaveLength(1);
      expect(parsed.sessions[0][0]).toBe(session2.id);
    });
  });

  describe('Session Listing', () => {
    it('should list all sessions', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();
      const session3 = await manager.createSession();

      const sessions = manager.listSessions();

      expect(sessions).toHaveLength(3);
      expect(sessions).toContain(session1);
      expect(sessions).toContain(session2);
      expect(sessions).toContain(session3);
    });

    it('should return empty array when no sessions', () => {
      const sessions = manager.listSessions();

      expect(sessions).toEqual([]);
    });
  });

  describe('LocalStorage Persistence', () => {
    it('should save sessions to localStorage', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      const stored = localStorage.getItem('terminal-sessions');
      expect(stored).toBeTruthy();

      const parsed = JSON.parse(stored!);
      expect(parsed.sessions).toHaveLength(2);
      expect(parsed.currentSessionId).toBe(session2.id);
    });

    it('should load sessions from localStorage', async () => {
      // Create and save sessions
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      // Create new manager instance
      const newManager = new SessionManager();
      newManager.loadFromStorage();

      // Verify sessions restored
      const sessions = newManager.listSessions();
      expect(sessions).toHaveLength(2);
      expect(newManager.getCurrentSession()?.id).toBe(session2.id);
    });

    it('should handle missing localStorage data gracefully', () => {
      localStorage.clear();

      manager.loadFromStorage();

      expect(manager.listSessions()).toEqual([]);
      expect(manager.getCurrentSession()).toBeNull();
    });

    it('should handle corrupted localStorage data', () => {
      localStorage.setItem('terminal-sessions', 'invalid json{');

      // Should not throw
      manager.loadFromStorage();

      expect(manager.listSessions()).toEqual([]);
    });

    it('should restore Date objects correctly', async () => {
      const session = await manager.createSession();

      // Create new manager and load
      const newManager = new SessionManager();
      newManager.loadFromStorage();

      const restored = newManager.getSession(session.id);
      expect(restored?.createdAt).toBeInstanceOf(Date);
    });
  });

  describe('Session State Management', () => {
    it('should maintain session state', async () => {
      const session = await manager.createSession();

      // Modify state
      session.state.workingDir = '/home/user';
      session.state.environment['PATH'] = '/usr/bin';
      session.state.history.push('ls -la');

      // Retrieve and verify
      const retrieved = manager.getSession(session.id);

      expect(retrieved?.state.workingDir).toBe('/home/user');
      expect(retrieved?.state.environment['PATH']).toBe('/usr/bin');
      expect(retrieved?.state.history).toContain('ls -la');
    });

    it('should persist state changes to localStorage', async () => {
      const session = await manager.createSession();

      // Modify and save
      session.state.workingDir = '/tmp';
      manager.switchSession(session.id); // Triggers save

      // Load in new manager
      const newManager = new SessionManager();
      newManager.loadFromStorage();

      const restored = newManager.getSession(session.id);
      expect(restored?.state.workingDir).toBe('/tmp');
    });
  });

  describe('Multiple Sessions', () => {
    it('should maintain multiple independent sessions', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();

      // Modify each session independently
      session1.state.workingDir = '/home/user1';
      session2.state.workingDir = '/home/user2';

      expect(manager.getSession(session1.id)?.state.workingDir).toBe('/home/user1');
      expect(manager.getSession(session2.id)?.state.workingDir).toBe('/home/user2');
    });

    it('should support switching between multiple sessions', async () => {
      const session1 = await manager.createSession();
      const session2 = await manager.createSession();
      const session3 = await manager.createSession();

      manager.switchSession(session1.id);
      expect(manager.getCurrentSession()).toBe(session1);

      manager.switchSession(session3.id);
      expect(manager.getCurrentSession()).toBe(session3);

      manager.switchSession(session2.id);
      expect(manager.getCurrentSession()).toBe(session2);
    });
  });

  describe('Edge Cases', () => {
    it('should handle rapid session creation', async () => {
      const sessions = await Promise.all([
        manager.createSession(),
        manager.createSession(),
        manager.createSession(),
      ]);

      expect(sessions).toHaveLength(3);
      expect(new Set(sessions.map(s => s.id)).size).toBe(3); // All unique IDs
    });

    it('should handle deletion of non-existent session', () => {
      // Should not throw
      manager.deleteSession('non-existent-id');

      expect(true).toBe(true);
    });

    it('should clear all sessions', async () => {
      await manager.createSession();
      await manager.createSession();
      await manager.createSession();

      manager.clearAllSessions();

      expect(manager.listSessions()).toEqual([]);
      expect(manager.getCurrentSession()).toBeNull();
      expect(localStorage.getItem('terminal-sessions')).toBeNull();
    });

    it('should handle empty environment and history', async () => {
      const session = await manager.createSession();

      expect(session.state.environment).toEqual({});
      expect(session.state.history).toEqual([]);
    });

    it('should handle very long session histories', async () => {
      const session = await manager.createSession();

      // Add 1000 commands
      for (let i = 0; i < 1000; i++) {
        session.state.history.push(`command-${i}`);
      }

      // Should still work
      expect(session.state.history).toHaveLength(1000);
    });
  });
});