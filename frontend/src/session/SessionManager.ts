/**
 * Session manager module - Handles terminal session state
 * Per spec-kit/004-frontend-spec.md section 3.3
 */

import type { SessionInfo } from '../types/index';

export class SessionManager {
  private currentSession: SessionInfo | null = null;

  /**
   * Create a new terminal session
   * TODO: Implement full session creation with server communication
   */
  async createSession(): Promise<SessionInfo> {
    // Temporary stub implementation
    // Full implementation will communicate with backend via WebSocket
    this.currentSession = {
      id: this.generateSessionId(),
      created: Date.now(),
      lastActivity: Date.now(),
    };

    return this.currentSession;
  }

  /**
   * Get current session info
   */
  getCurrentSession(): SessionInfo | null {
    return this.currentSession;
  }

  /**
   * Generate a unique session ID
   * TODO: Replace with server-generated ID
   */
  private generateSessionId(): string {
    return `session-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
}