# E2E Test Suite - Web Terminal

Comprehensive end-to-end test suite for the web-terminal project using Playwright.

**Per spec-kit/008-testing-spec.md**: These tests validate critical user workflows from browser to backend.

## 🎯 Test Coverage

### 1. Terminal Operations (`terminal.spec.ts`) - 728 lines
**Status**: ✅ Existing

Tests core terminal functionality:
- Basic rendering and loading
- Single-port architecture verification (WebSocket URL construction)
- User input and keyboard interactions (typing, Enter, Ctrl+C, Ctrl+V, Ctrl+F)
- WebSocket connection management and reconnection
- Terminal resize handling
- Session management and persistence
- Error handling and edge cases
- Mobile and responsive behavior
- Performance and resource usage

**Critical Workflows**:
- Terminal connection and command execution
- Dynamic URL construction from `window.location` (single-port)
- No hardcoded ports in source code

---

### 2. Authentication (`authentication.spec.ts`) - 525 lines
**Status**: ✅ Created

Tests authentication and security:
- JWT token authentication (accept/reject)
- WebSocket URL construction with token query parameter
- Token expiration and refresh
- Session management with authentication
- Secure WebSocket connection (ws:// vs wss://)
- Authentication error handling
- Rate limiting and security
- Cross-tab authentication sync

**Critical Workflows**:
- JWT token validation via WebSocket query parameter
- Token not exposed in page URL (security)
- Authentication failure handling (code 4000, 4001)
- Token refresh on expiration

---

### 3. File Operations (`file-operations.spec.ts`) - 609 lines
**Status**: ✅ Created

Tests file upload/download workflows:
- Single and multiple file uploads
- Large file handling with chunking (64KB chunks)
- File checksum calculation and verification
- Upload/download progress tracking
- File transfer cancellation
- Binary file handling and integrity
- Concurrent file operations
- File management (list, delete, metadata)

**Critical Workflows**:
- Chunked file upload with progress (per 007-websocket-spec.md)
- SHA-256 checksum validation
- Binary data integrity preservation
- Error handling (network interruption, checksum mismatch, quota exceeded)

---

### 4. Multi-Session (`multi-session.spec.ts`) - 635 lines
**Status**: ✅ Created

Tests concurrent session management:
- Multiple independent session creation
- Session switching and state preservation
- Session isolation (environment variables, working directory, processes, history)
- Concurrent command execution across sessions
- Session lifecycle management (creation, activity tracking, cleanup)
- Session resource limits and tracking
- Error recovery per session

**Critical Workflows**:
- Each session has unique ID
- Sessions are completely isolated from each other
- Session state persists across page reload
- Independent error recovery (one session failure doesn't affect others)

---

### 5. Error Handling (`error-handling.spec.ts`) - 683 lines
**Status**: ✅ Created

Tests comprehensive error handling:
- Network error handling (offline, slow network, timeout, interruption recovery)
- Server error handling (command errors, permissions, internal errors, session expiration)
- Input validation errors (invalid JSON, long commands, special characters, malicious input)
- Resource limit errors (quota, memory, processes, timeout)
- Error recovery mechanisms (automatic retry, exponential backoff, state restoration)
- User-friendly error messages
- Critical error handling without crashes

**Critical Workflows**:
- Graceful degradation during network failure
- Command queuing during disconnection
- Exponential backoff for reconnection
- Clear error messages with actionable suggestions
- Error codes per 007-websocket-spec.md (COMMAND_FAILED, PERMISSION_DENIED, etc.)

---

## 🏗️ Architecture

### Single-Port Architecture Compliance

**All tests verify**:
- WebSocket URLs constructed dynamically from `window.location`
- No hardcoded port numbers in source code
- Protocol auto-detection (`ws://` for http, `wss://` for https)
- All resources served from same origin

**Example**:
```typescript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const host = window.location.host; // includes port
const url = `${protocol}//${host}/ws?token=${token}`;
```

### Test Configuration

**Playwright Config** (`playwright.config.ts`):
- Base URL: `http://localhost:8080` (configurable via `TEST_BASE_URL`)
- Single-port server in CI (no separate frontend dev server)
- Multiple browser testing (Chromium, Firefox, Safari, Mobile)
- Automatic retry on CI (2 retries)
- Screenshot and video on failure

---

## 🚀 Running Tests

### Run All E2E Tests
```bash
pnpm run test:e2e
```

### Run Specific Test Suite
```bash
pnpm run test:e2e tests/e2e/terminal.spec.ts
pnpm run test:e2e tests/e2e/authentication.spec.ts
pnpm run test:e2e tests/e2e/file-operations.spec.ts
pnpm run test:e2e tests/e2e/multi-session.spec.ts
pnpm run test:e2e tests/e2e/error-handling.spec.ts
```

### Debug Mode (Interactive)
```bash
pnpm run test:e2e:debug
```

### UI Mode (Visual Test Runner)
```bash
pnpm run test:e2e:ui
```

### Run on Specific Browser
```bash
pnpm run test:e2e -- --project=chromium
pnpm run test:e2e -- --project=firefox
pnpm run test:e2e -- --project=webkit
```

### CI Environment
```bash
# Server must be running on port 8080
TEST_BASE_URL=http://localhost:8080 pnpm run test:e2e
```

---

## 📁 Test Fixtures

Located in `tests/fixtures/`:

- **test.txt**: Simple text file (~100 bytes)
- **large-file.txt**: Large text file (~10KB) for chunked transfers
- **binary-test.bin**: Binary file for binary data handling
- **README.md**: Fixtures documentation

**Usage**:
```typescript
import { join } from 'path';

const fixturesPath = join(__dirname, '../fixtures');
const testFile = join(fixturesPath, 'test.txt');
```

---

## 🎯 Test Patterns

### 1. WebSocket Mocking
Tests use `page.addInitScript()` to mock WebSocket connections:
```typescript
async function setupWebSocketMock(page: Page) {
  await page.addInitScript(() => {
    class MockWebSocket extends EventTarget { /* ... */ }
    window.WebSocket = MockWebSocket;
  });
}
```

### 2. Page Evaluation
Tests execute code in browser context:
```typescript
const result = await page.evaluate(() => {
  // Code runs in browser
  return window.location.host;
});
```

### 3. Multi-Context Testing
Tests use multiple browser contexts for session isolation:
```typescript
const context = await browser.newContext();
const page1 = await context.newPage();
const page2 = await context.newPage();
```

### 4. Network Simulation
Tests simulate network conditions:
```typescript
await page.context().setOffline(true); // Offline
await page.route('**/*', async (route) => {
  await new Promise(resolve => setTimeout(resolve, 500)); // Slow
  await route.continue();
});
```

---

## ✅ Acceptance Criteria

Per spec-kit/008-testing-spec.md, E2E tests validate:

1. ✅ Terminal connection and command execution
2. ✅ Authentication with JWT tokens
3. ✅ File upload and download with chunking
4. ✅ Multiple concurrent sessions
5. ✅ Error handling and recovery
6. ✅ Terminal resize and responsiveness
7. ✅ Single-port architecture compliance
8. ✅ WebSocket reconnection with exponential backoff
9. ✅ Session persistence and restoration
10. ✅ Security (no token exposure, input sanitization)

---

## 📊 Test Statistics

| Test Suite | Lines | Tests | Coverage |
|------------|-------|-------|----------|
| terminal.spec.ts | 728 | 20+ | Core terminal functionality |
| authentication.spec.ts | 525 | 25+ | JWT auth and security |
| file-operations.spec.ts | 609 | 30+ | File upload/download |
| multi-session.spec.ts | 635 | 25+ | Concurrent sessions |
| error-handling.spec.ts | 683 | 40+ | Error recovery |
| **Total** | **3,180** | **140+** | **All critical user workflows** |

---

## 🔒 Security Testing

Tests validate security requirements:
- ✅ Unauthenticated WebSocket connections rejected (code 4000)
- ✅ Token not exposed in browser URL or history
- ✅ Input sanitization (XSS, SQL injection, path traversal)
- ✅ Blocked dangerous commands (`rm -rf /`, etc.)
- ✅ Path traversal prevention (`../../../etc/passwd`)
- ✅ Rate limiting enforcement (code 4002)

---

## 🚨 Critical Requirements

**MANDATORY per spec-kit**:

1. **Single-Port Architecture**:
   - ALL WebSocket URLs constructed from `window.location`
   - NO hardcoded port numbers in source code
   - HTTP and WebSocket on SAME port (default: 8080)

2. **WebSocket Protocol**:
   - Follows spec-kit/007-websocket-spec.md exactly
   - Error codes match specification
   - Message format validated

3. **GitHub Actions CI**:
   - Tests run in CI environment with real server
   - All tests must pass before merge
   - Backend server on port 8080 (single-port)

---

## 📝 Writing New Tests

1. Follow existing patterns in test files
2. Use descriptive test names: `should <expected behavior> when <scenario>`
3. Verify single-port architecture compliance
4. Mock WebSocket connections for isolated testing
5. Test both happy path and error cases
6. Add fixtures to `tests/fixtures/` if needed
7. Document in this README

---

## 🐛 Debugging

### View Test Reports
```bash
pnpm playwright show-report
```

### Debug Failed Test
```bash
pnpm run test:e2e:debug tests/e2e/terminal.spec.ts
```

### Run with Verbose Logging
```bash
DEBUG=pw:api pnpm run test:e2e
```

### Inspect WebSocket Traffic
```bash
# Enable WebSocket frame logging
DEBUG=pw:websocket pnpm run test:e2e
```

---

## 📚 Related Documentation

- [Testing Specification](/docs/spec-kit/008-testing-spec.md)
- [Frontend Specification](/docs/spec-kit/004-frontend-spec.md)
- [WebSocket Protocol](/docs/spec-kit/007-websocket-spec.md)
- [Backend Specification](/docs/spec-kit/003-backend-spec.md)

---

## 🎉 Summary

**Comprehensive E2E test suite created with 3,180+ lines and 140+ tests covering:**
- ✅ Terminal interaction and WebSocket communication
- ✅ JWT authentication and security
- ✅ File upload/download with chunking
- ✅ Multiple concurrent sessions
- ✅ Error handling and recovery
- ✅ Single-port architecture compliance
- ✅ All critical user workflows

**All tests follow spec-kit requirements and are ready for CI integration!**