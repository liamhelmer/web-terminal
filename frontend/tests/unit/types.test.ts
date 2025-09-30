/**
 * Unit tests for type guards and utilities
 * Per spec-kit/008-testing-spec.md
 */

import { describe, it, expect } from 'vitest';
import {
  isClientMessage,
  isServerMessage,
  isErrorMessage,
  isOutputMessage,
  isProcessExitedMessage,
  type ClientMessage,
  type ServerMessage,
  type ErrorMessage,
  type OutputMessage,
  type ProcessExitedMessage,
  ErrorCodes,
  Signals,
  CloseCodes,
} from '../../src/types/index';

describe('Type Guards', () => {
  describe('isClientMessage', () => {
    it('should return true for valid command message', () => {
      const msg: ClientMessage = { type: 'command', data: 'ls' };
      expect(isClientMessage(msg)).toBe(true);
    });

    it('should return true for valid resize message', () => {
      const msg: ClientMessage = { type: 'resize', cols: 80, rows: 24 };
      expect(isClientMessage(msg)).toBe(true);
    });

    it('should return true for valid signal message', () => {
      const msg: ClientMessage = { type: 'signal', signal: 'SIGINT' };
      expect(isClientMessage(msg)).toBe(true);
    });

    it('should return false for null', () => {
      expect(isClientMessage(null)).toBe(false);
    });

    it('should return false for undefined', () => {
      expect(isClientMessage(undefined)).toBe(false);
    });

    it('should return false for primitive types', () => {
      expect(isClientMessage('string')).toBe(false);
      expect(isClientMessage(123)).toBe(false);
      expect(isClientMessage(true)).toBe(false);
    });

    it('should return false for object without type', () => {
      expect(isClientMessage({ data: 'test' })).toBe(false);
    });

    it('should return false for object with non-string type', () => {
      expect(isClientMessage({ type: 123 })).toBe(false);
    });
  });

  describe('isServerMessage', () => {
    it('should return true for valid output message', () => {
      const msg: ServerMessage = { type: 'output', stream: 'stdout', data: 'test' };
      expect(isServerMessage(msg)).toBe(true);
    });

    it('should return true for valid error message', () => {
      const msg: ServerMessage = {
        type: 'error',
        code: 'COMMAND_FAILED',
        message: 'Command failed',
      };
      expect(isServerMessage(msg)).toBe(true);
    });

    it('should return true for valid process_started message', () => {
      const msg: ServerMessage = {
        type: 'process_started',
        pid: 1234,
        command: 'ls',
      };
      expect(isServerMessage(msg)).toBe(true);
    });

    it('should return false for null', () => {
      expect(isServerMessage(null)).toBe(false);
    });

    it('should return false for undefined', () => {
      expect(isServerMessage(undefined)).toBe(false);
    });

    it('should return false for primitive types', () => {
      expect(isServerMessage('string')).toBe(false);
      expect(isServerMessage(123)).toBe(false);
      expect(isServerMessage(true)).toBe(false);
    });

    it('should return false for object without type', () => {
      expect(isServerMessage({ data: 'test' })).toBe(false);
    });
  });

  describe('isErrorMessage', () => {
    it('should return true for error message', () => {
      const msg: ErrorMessage = {
        type: 'error',
        code: 'COMMAND_FAILED',
        message: 'Failed',
      };
      expect(isErrorMessage(msg)).toBe(true);
    });

    it('should return false for output message', () => {
      const msg: OutputMessage = {
        type: 'output',
        stream: 'stdout',
        data: 'test',
      };
      expect(isErrorMessage(msg)).toBe(false);
    });

    it('should return false for process_exited message', () => {
      const msg: ProcessExitedMessage = {
        type: 'process_exited',
        pid: 1234,
        exit_code: 0,
        signal: null,
      };
      expect(isErrorMessage(msg)).toBe(false);
    });
  });

  describe('isOutputMessage', () => {
    it('should return true for output message', () => {
      const msg: OutputMessage = {
        type: 'output',
        stream: 'stdout',
        data: 'test',
      };
      expect(isOutputMessage(msg)).toBe(true);
    });

    it('should return false for error message', () => {
      const msg: ErrorMessage = {
        type: 'error',
        code: 'COMMAND_FAILED',
        message: 'Failed',
      };
      expect(isOutputMessage(msg)).toBe(false);
    });
  });

  describe('isProcessExitedMessage', () => {
    it('should return true for process_exited message', () => {
      const msg: ProcessExitedMessage = {
        type: 'process_exited',
        pid: 1234,
        exit_code: 0,
        signal: null,
      };
      expect(isProcessExitedMessage(msg)).toBe(true);
    });

    it('should return false for output message', () => {
      const msg: OutputMessage = {
        type: 'output',
        stream: 'stdout',
        data: 'test',
      };
      expect(isProcessExitedMessage(msg)).toBe(false);
    });
  });
});

describe('Constants', () => {
  describe('ErrorCodes', () => {
    it('should have all error codes defined', () => {
      expect(ErrorCodes.COMMAND_NOT_FOUND).toBe('COMMAND_NOT_FOUND');
      expect(ErrorCodes.COMMAND_FAILED).toBe('COMMAND_FAILED');
      expect(ErrorCodes.COMMAND_TIMEOUT).toBe('COMMAND_TIMEOUT');
      expect(ErrorCodes.COMMAND_KILLED).toBe('COMMAND_KILLED');
      expect(ErrorCodes.PERMISSION_DENIED).toBe('PERMISSION_DENIED');
      expect(ErrorCodes.PATH_NOT_FOUND).toBe('PATH_NOT_FOUND');
      expect(ErrorCodes.PATH_INVALID).toBe('PATH_INVALID');
      expect(ErrorCodes.SESSION_EXPIRED).toBe('SESSION_EXPIRED');
      expect(ErrorCodes.RESOURCE_LIMIT).toBe('RESOURCE_LIMIT');
      expect(ErrorCodes.QUOTA_EXCEEDED).toBe('QUOTA_EXCEEDED');
      expect(ErrorCodes.INVALID_MESSAGE).toBe('INVALID_MESSAGE');
      expect(ErrorCodes.INTERNAL_ERROR).toBe('INTERNAL_ERROR');
    });

    it('should have string literal types', () => {
      const code: 'COMMAND_FAILED' = ErrorCodes.COMMAND_FAILED;
      expect(code).toBe('COMMAND_FAILED');
    });
  });

  describe('Signals', () => {
    it('should have all signal types defined', () => {
      expect(Signals.SIGINT).toBe('SIGINT');
      expect(Signals.SIGTERM).toBe('SIGTERM');
      expect(Signals.SIGKILL).toBe('SIGKILL');
    });

    it('should have string literal types', () => {
      const signal: 'SIGINT' = Signals.SIGINT;
      expect(signal).toBe('SIGINT');
    });
  });

  describe('CloseCodes', () => {
    it('should have all close codes defined', () => {
      expect(CloseCodes.NORMAL_CLOSURE).toBe(1000);
      expect(CloseCodes.GOING_AWAY).toBe(1001);
      expect(CloseCodes.PROTOCOL_ERROR).toBe(1002);
      expect(CloseCodes.UNSUPPORTED_DATA).toBe(1003);
      expect(CloseCodes.POLICY_VIOLATION).toBe(1008);
      expect(CloseCodes.INTERNAL_ERROR).toBe(1011);
      expect(CloseCodes.AUTHENTICATION_FAILED).toBe(4000);
      expect(CloseCodes.SESSION_EXPIRED).toBe(4001);
      expect(CloseCodes.RATE_LIMIT).toBe(4002);
    });

    it('should have numeric literal types', () => {
      const code: 1000 = CloseCodes.NORMAL_CLOSURE;
      expect(code).toBe(1000);
    });
  });
});

describe('Message Validation', () => {
  describe('ClientMessage types', () => {
    it('should validate command message structure', () => {
      const msg: ClientMessage = { type: 'command', data: 'ls -la' };
      expect(msg.type).toBe('command');
      expect(msg).toHaveProperty('data');
    });

    it('should validate resize message structure', () => {
      const msg: ClientMessage = { type: 'resize', cols: 120, rows: 40 };
      expect(msg.type).toBe('resize');
      expect(msg).toHaveProperty('cols');
      expect(msg).toHaveProperty('rows');
    });

    it('should validate signal message structure', () => {
      const msg: ClientMessage = { type: 'signal', signal: 'SIGTERM' };
      expect(msg.type).toBe('signal');
      expect(msg).toHaveProperty('signal');
    });

    it('should validate ping message structure', () => {
      const msg: ClientMessage = { type: 'ping', timestamp: Date.now() };
      expect(msg.type).toBe('ping');
      expect(msg).toHaveProperty('timestamp');
    });

    it('should validate echo message structure', () => {
      const msg: ClientMessage = { type: 'echo', data: 'test' };
      expect(msg.type).toBe('echo');
      expect(msg).toHaveProperty('data');
    });
  });

  describe('ServerMessage types', () => {
    it('should validate output message structure', () => {
      const msg: ServerMessage = {
        type: 'output',
        stream: 'stdout',
        data: 'output',
      };
      expect(msg.type).toBe('output');
      expect(msg).toHaveProperty('stream');
      expect(msg).toHaveProperty('data');
    });

    it('should validate error message structure', () => {
      const msg: ServerMessage = {
        type: 'error',
        code: 'COMMAND_FAILED',
        message: 'Error',
      };
      expect(msg.type).toBe('error');
      expect(msg).toHaveProperty('code');
      expect(msg).toHaveProperty('message');
    });

    it('should validate process_started message structure', () => {
      const msg: ServerMessage = {
        type: 'process_started',
        pid: 1234,
        command: 'ls',
      };
      expect(msg.type).toBe('process_started');
      expect(msg).toHaveProperty('pid');
      expect(msg).toHaveProperty('command');
    });

    it('should validate process_exited message structure', () => {
      const msg: ServerMessage = {
        type: 'process_exited',
        pid: 1234,
        exit_code: 0,
        signal: null,
      };
      expect(msg.type).toBe('process_exited');
      expect(msg).toHaveProperty('pid');
      expect(msg).toHaveProperty('exit_code');
      expect(msg).toHaveProperty('signal');
    });

    it('should validate flow_control message structure', () => {
      const msg: ServerMessage = {
        type: 'flow_control',
        action: 'pause',
      };
      expect(msg.type).toBe('flow_control');
      expect(msg).toHaveProperty('action');
    });

    it('should validate pong message structure', () => {
      const msg: ServerMessage = {
        type: 'pong',
        timestamp: Date.now(),
        latency_ms: 42,
      };
      expect(msg.type).toBe('pong');
      expect(msg).toHaveProperty('timestamp');
      expect(msg).toHaveProperty('latency_ms');
    });
  });
});
