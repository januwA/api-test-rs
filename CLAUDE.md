# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **API Test** - a cross-platform desktop API testing tool built with **Tauri 2.0 + React + TypeScript**. It was migrated from an egui-based Rust application to a modern web-based UI while preserving all core business logic in Rust.

The application supports:
- HTTP/HTTPS requests with various body types (Raw, Form, FormData)
- Pre-request and post-response scripting using the Rhai script engine
- Variable substitution and management
- Batch/concurrent requests (up to 10,000 concurrent)
- WebSocket connections (in progress)
- Project and test organization in groups
- cURL command import/export

## Architecture

### Frontend-Backend Communication

The app uses Tauri's IPC system:
- **Frontend (React)**: UI components call Rust functions via `@tauri-apps/api` invoke
- **Backend (Rust)**: Tauri commands in `src-tauri/src/commands.rs` expose functionality
- **State**: Shared state managed via `AppState` (Mutex-wrapped Project and script output)

### Key Backend Components

1. **src-tauri/src/lib.rs**: Core business logic including:
   - `HttpRequestConfig`, `HttpResponse`, `Project` data structures
   - HTTP client implementation (reqwest-based)
   - Variable substitution in URLs, headers, bodies

2. **src-tauri/src/script_engine.rs**: Rhai-based scripting engine
   - Pre-request scripts: Modify requests before sending (URL, headers, body, variables)
   - Post-response scripts: Process responses and extract data to variables
   - Provides `print()`, `log()`, crypto functions (md5, sha256, base64, etc.)

3. **src-tauri/src/commands.rs**: Tauri command API
   - Project management: load, save, create, update
   - HTTP requests: `send_http_request`, `send_http_batch`
   - Variables: get, set, delete
   - Utilities: `generate_curl_command`

4. **src-tauri/src/util.rs**: Helper functions
   - File I/O for project persistence
   - Variable interpolation (`{{variable_name}}` syntax)
   - Request building and execution

### Frontend Structure

- **ui/src/stores/**: Zustand state management
  - `projectStore.ts`: Project, groups, tests, variables
  - `responseStore.ts`: HTTP responses and statistics

- **ui/src/components/**:
  - `Layout.tsx`: Main app layout
  - `TopMenu.tsx`: File operations, save/load
  - `Sidebar.tsx`: Group/test tree navigation
  - `RequestPanel.tsx`: Request configuration (tabs: Params, Headers, Body, Scripts, Curl)
  - `ResponsePanel.tsx`: Response display (tabs: Data, Header, Stats)
  - `VariablesPanel.tsx`: Global variables management
  - `common/`: Reusable components (Tabs, CodeEditor, PairTable)

- **ui/src/types/index.ts**: TypeScript type definitions matching Rust structs

### Data Flow

1. User selects a test in Sidebar → `projectStore.selectTest()`
2. RequestPanel displays test configuration → user modifies request
3. User clicks "Send" → `invoke("send_http_request", { config, variables })`
4. Backend executes:
   - Run pre-request script (if enabled)
   - Substitute variables in URL/headers/body
   - Send HTTP request
   - Run post-response script (if enabled)
   - Return response
5. ResponsePanel displays results → `responseStore` updates

## Development Commands

### Setup
```bash
npm install
```

### Development Mode
```bash
npm run tauri:dev
```
This starts the Vite dev server (port 5173) and launches the Tauri window.

### Build Release
```bash
npm run tauri:build
```
Builds optimized frontend and creates platform-specific installers in `src-tauri/target/release/bundle/`

### Frontend Only (for UI development)
```bash
npm run dev
```

### Rust Backend Check
```bash
cd src-tauri
cargo check
cargo test
```

### Build Frontend
```bash
npm run build
```
Output to `ui/dist/` (configured in vite.config.ts)

## Project File Format

Projects are saved as JSON files with structure:
```json
{
  "name": "Project Name",
  "groups": [
    {
      "name": "Group Name",
      "children": [/* HttpTest objects */]
    }
  ],
  "variables": [/* PairUi objects */]
}
```

Config file (`.api-test.json`) stores:
- `project_path`: Last opened project
- `font_size`: UI font size

## Variable Substitution

Variables use `{{variable_name}}` syntax and are substituted in:
- Request URLs
- Headers
- Query parameters
- Request body (all types)

Variables can be:
- Defined globally in the Variables panel
- Modified by pre-request scripts
- Extracted by post-response scripts using `set_var(key, value)`

## Script Engine (Rhai)

### Pre-Request Script Context
Available objects:
- `url` (String): Modify request URL
- `method` (String): HTTP method
- `headers` (Map): Request headers
- `params` (Map): Query parameters
- `body` (String): Request body
- `variables` (Map): Environment variables

Functions:
- `set_var(key, value)`: Update variable
- `get_var(key)`: Get variable
- `print(msg)`: Output to script console
- `log(msg)`: Same as print
- Crypto: `md5()`, `sha256()`, `base64_encode()`, `base64_decode()`, `hmac_sha256()`

### Post-Response Script Context
Additional objects:
- `status` (i64): Response status code
- `response_headers` (Map): Response headers
- `response_body` (String): Response body
- `duration` (i64): Response time in ms

Common pattern: Extract JSON fields to variables
```rhai
let json = parse_json(response_body);
set_var("token", json["access_token"]);
set_var("user_id", json["user"]["id"]);
```

## Batch Requests

The `send_http_batch` command sends multiple requests concurrently:
- Maximum 10,000 concurrent requests (configurable in commands.rs:116)
- Uses Tokio `FuturesUnordered` for async execution
- Returns array of all responses
- UI can display statistics: success rate, average latency, QPS

## UI Framework

- **Styling**: Tailwind CSS (configured in tailwind.config.js)
- **Icons**: Unicode emoji (ensure emoji font support)
- **Code Editor**: Monaco Editor (`@monaco-editor/react`) for scripts and JSON
- **State**: Zustand (simpler than Redux, no boilerplate)

## Common Development Tasks

### Adding a New Tauri Command

1. Add function in `src-tauri/src/commands.rs`:
```rust
#[tauri::command]
pub async fn my_command(param: String) -> Result<String, String> {
    Ok(param)
}
```

2. Register in `src-tauri/src/main.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    my_command
])
```

3. Call from frontend:
```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke<string>("my_command", { param: "value" });
```

### Adding a New UI Component

1. Create component in `ui/src/components/MyComponent.tsx`
2. Import and use in parent component
3. Update types in `ui/src/types/index.ts` if needed
4. Add to Zustand store if state management is needed

### Modifying Request/Response Types

1. Update Rust structs in `src-tauri/src/lib.rs`
2. Update TypeScript interfaces in `ui/src/types/index.ts`
3. Ensure serialization works (Rust: `#[derive(Serialize, Deserialize)]`, TS: exact field names)

## Known Issues & TODO

- WebSocket functionality is stubbed (commands exist but not implemented)
- Curl command parsing (`parse_curl_command`) returns default config
- Response panel Stats tab needs full implementation (success rate charts, QPS graphs)
- Test runner for scripts would be useful

## Important Notes

- **Binary compatibility**: Old project files from the egui version are fully compatible
- **Windows paths**: When using fs operations, handle Windows path separators correctly
- **Async contexts**: All Tauri commands are async; use `.await` when calling async Rust functions
- **State locking**: Always check `Mutex::lock()` results and handle errors
- **Script security**: Rhai scripts run in a sandboxed environment without file/network access (except via provided functions)
