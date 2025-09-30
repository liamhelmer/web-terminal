# Test Fixtures

This directory contains test fixtures for E2E tests.

## Files

### test.txt
Simple text file for basic upload/download tests.
- Size: ~100 bytes
- Type: text/plain
- Usage: Basic file operations

### large-file.txt
Larger text file for testing chunked transfers.
- Size: ~10KB
- Type: text/plain
- Usage: Chunked upload/download, progress tracking

### binary-test.bin
Binary file for testing binary data handling.
- Size: Small
- Type: application/octet-stream
- Usage: Binary file upload/download

## Usage in Tests

Import fixtures in E2E tests:

```typescript
import { readFileSync } from 'fs';
import { join } from 'path';

const fixturesPath = join(__dirname, '../fixtures');
const testFile = join(fixturesPath, 'test.txt');
```

## Adding New Fixtures

1. Create file in this directory
2. Document it in this README
3. Reference in relevant test spec