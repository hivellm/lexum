## Why

Lexum must support multiple protocols beyond HTTP to integrate with AI/LLM systems (MCP), enable high-performance communication (UMICP), support streaming responses (StreamableHTTP), and provide real-time updates (WebSocket). This makes Lexum a versatile search platform for diverse use cases.

## What Changes

- Implement StreamableHTTP with Server-Sent Events (SSE)
- Add MCP (Model Context Protocol) handler for AI integration
- Implement UMICP (Universal Model Interchange Communication Protocol) binary protocol
- Add WebSocket support for real-time updates
- Implement protocol detection and routing
- Add streaming result support
- Implement backpressure handling

## Impact

- Affected specs: `streamable-http`, `mcp-protocol`, `umicp-protocol`, `websocket-support`
- Affected code: Creates `lexum-server/src/api/`:
  - `stream/` - StreamableHTTP
  - `mcp/` - MCP handler
  - `umicp/` - UMICP handler
  - `ws/` - WebSocket
- Dependencies: tokio-tungstenite (WebSocket), bincode (UMICP), futures
- Must integrate with existing REST API

