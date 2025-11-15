# Cursor MCP Configuration

This directory contains MCP (Model Context Protocol) configuration for Cursor IDE.

## Setup

To use the Lexum MCP server in Cursor:

1. Make sure the Lexum server is running on `http://localhost:17000`
2. Add the MCP configuration to your Cursor settings

The MCP server provides the following tools:
- `search` - Search documents in an index
- `retrieve` - Retrieve a specific document by ID
- `aggregate` - Perform aggregations on search results
- `list_indices` - List all available indices

## Configuration

The MCP server endpoint is available at: `http://localhost:17000/mcp`

You can configure it in Cursor's settings by adding:

```json
{
  "mcpServers": {
    "Lexum": {
      "url": "http://localhost:17000/mcp",
      "type": "streamableHttp",
      "protocol": "http"
    }
  }
}
```

## Usage

Once configured, you can use MCP tools in Cursor to interact with your Lexum search engine.

