/**
 * TypeScript type definitions
 * Per spec-kit/004-frontend-spec.md section 4
 */

export interface SessionInfo {
  id: string;
  created: number;
  lastActivity: number;
}

export interface WebSocketMessage {
  type: string;
  data: unknown;
}

// Additional types will be added as implementation progresses