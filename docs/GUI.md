# Lexum GUI

Electron-based graphical user interface for Lexum search engine, inspired by Kibana.

## Overview

Lexum GUI provides a modern, intuitive interface for:

- **Search & Discover**: Interactive search and data exploration
- **Observability**: Real-time monitoring and dashboards
- **Index Management**: Create, configure, and manage indices
- **Security**: User and role management
- **Logs**: Centralized log viewing and analysis
- **Dev Tools**: LQL query console and API testing

## Architecture

```
┌──────────────────────────────────────────┐
│         Electron Main Process            │
│  - Window Management                     │
│  - Menu Bar                              │
│  - IPC Communication                     │
└────────────────┬─────────────────────────┘
                 │
┌────────────────┴─────────────────────────┐
│      Electron Renderer Process           │
│  ┌────────────────────────────────────┐  │
│  │         React Application          │  │
│  │  - TypeScript                      │  │
│  │  - React Router                    │  │
│  │  - Redux (State Management)        │  │
│  │  - Material-UI Components          │  │
│  └────────────────────────────────────┘  │
└────────────────┬─────────────────────────┘
                 │
                 │ HTTP/WebSocket
                 ▼
┌──────────────────────────────────────────┐
│          Lexum Backend                   │
│  - REST API                              │
│  - WebSocket (Real-time)                 │
│  - MCP/UMICP                             │
└──────────────────────────────────────────┘
```

## Technology Stack

### Core
- **Electron**: 28.0+
- **React**: 18.0+
- **TypeScript**: 5.0+
- **Vite**: Build tool

### UI Components
- **Material-UI (MUI)**: Component library
- **Monaco Editor**: Code editor for LQL
- **Recharts**: Charting library
- **D3.js**: Advanced visualizations
- **React Grid Layout**: Dashboard layout

### State Management
- **Redux Toolkit**: Global state
- **React Query**: Server state
- **Zustand**: Local state

### Networking
- **Axios**: HTTP client
- **Socket.io**: WebSocket client
- **EventSource**: SSE for streaming

## Installation

### From Release

```bash
# Download for your platform
# macOS
https://github.com/your-org/lexum-gui/releases/download/v0.1.0/Lexum-0.1.0.dmg

# Windows
https://github.com/your-org/lexum-gui/releases/download/v0.1.0/Lexum-Setup-0.1.0.exe

# Linux
https://github.com/your-org/lexum-gui/releases/download/v0.1.0/Lexum-0.1.0.AppImage
```

### From Source

```bash
# Clone repository
git clone https://github.com/your-org/lexum-gui
cd lexum-gui

# Install dependencies
npm install

# Development mode
npm run dev

# Build for production
npm run build

# Package for distribution
npm run package
```

## Project Structure

```
lexum-gui/
├── src/
│   ├── main/              # Electron main process
│   │   ├── index.ts
│   │   ├── ipc.ts
│   │   └── menu.ts
│   ├── renderer/          # React application
│   │   ├── App.tsx
│   │   ├── main.tsx
│   │   ├── components/    # React components
│   │   ├── pages/         # Page components
│   │   ├── store/         # Redux store
│   │   ├── api/           # API clients
│   │   ├── hooks/         # Custom hooks
│   │   └── utils/         # Utilities
│   └── preload/           # Preload scripts
│       └── index.ts
├── public/                # Static assets
├── build/                 # Build configuration
├── package.json
└── tsconfig.json
```

## Features

### 1. Home Dashboard

Landing page with cluster overview.

**Components:**
- Cluster health indicator
- Node status cards
- Recent activity feed
- Quick actions
- System metrics overview

```tsx
// components/HomeDashboard.tsx
import { Grid, Card, CardContent } from '@mui/material';
import { ClusterHealth } from './ClusterHealth';
import { NodeStatus } from './NodeStatus';
import { RecentActivity } from './RecentActivity';

export const HomeDashboard: React.FC = () => {
  return (
    <Grid container spacing={3}>
      <Grid item xs={12}>
        <ClusterHealth />
      </Grid>
      <Grid item xs={12} md={6}>
        <NodeStatus />
      </Grid>
      <Grid item xs={12} md={6}>
        <RecentActivity />
      </Grid>
    </Grid>
  );
};
```

### 2. Discover (Search & Explore)

Interactive search interface.

**Features:**
- Query bar with autocomplete
- Filter builder
- Field selector
- Result table with sorting
- Time range picker
- Save/load searches
- Export results

```tsx
// pages/Discover.tsx
import { useState } from 'react';
import { QueryBar } from '../components/QueryBar';
import { FilterPanel } from '../components/FilterPanel';
import { ResultsTable } from '../components/ResultsTable';
import { TimeRangePicker } from '../components/TimeRangePicker';

export const Discover: React.FC = () => {
  const [query, setQuery] = useState('');
  const [filters, setFilters] = useState([]);
  const [timeRange, setTimeRange] = useState({ from: 'now-15m', to: 'now' });

  return (
    <div className="discover-page">
      <QueryBar value={query} onChange={setQuery} />
      <TimeRangePicker value={timeRange} onChange={setTimeRange} />
      <div className="discover-content">
        <FilterPanel filters={filters} onChange={setFilters} />
        <ResultsTable query={query} filters={filters} timeRange={timeRange} />
      </div>
    </div>
  );
};
```

### 3. Visualizations & Dashboards

Create and view visualizations.

**Visualization Types:**
- Line charts (time series)
- Bar charts
- Pie charts
- Data tables
- Metric displays
- Heat maps
- Tag clouds

```tsx
// components/VisualizationBuilder.tsx
import { Line, Bar, Pie } from 'recharts';
import { useState } from 'react';

interface VisualizationProps {
  type: 'line' | 'bar' | 'pie';
  data: any[];
  config: VisualizationConfig;
}

export const Visualization: React.FC<VisualizationProps> = ({ 
  type, 
  data, 
  config 
}) => {
  switch (type) {
    case 'line':
      return <LineChart data={data} {...config} />;
    case 'bar':
      return <BarChart data={data} {...config} />;
    case 'pie':
      return <PieChart data={data} {...config} />;
  }
};
```

**Dashboard Grid:**
```tsx
// pages/Dashboard.tsx
import GridLayout from 'react-grid-layout';
import { Visualization } from '../components/Visualization';

export const Dashboard: React.FC = () => {
  const [layout, setLayout] = useState([
    { i: 'a', x: 0, y: 0, w: 6, h: 4 },
    { i: 'b', x: 6, y: 0, w: 6, h: 4 },
    { i: 'c', x: 0, y: 4, w: 12, h: 4 },
  ]);

  return (
    <GridLayout
      layout={layout}
      onLayoutChange={setLayout}
      cols={12}
      rowHeight={30}
      width={1200}
    >
      <div key="a"><Visualization type="line" {...} /></div>
      <div key="b"><Visualization type="pie" {...} /></div>
      <div key="c"><Visualization type="bar" {...} /></div>
    </GridLayout>
  );
};
```

### 4. Dev Tools

Developer console for testing queries.

**Features:**
- LQL query editor (Monaco)
- Syntax highlighting
- Autocomplete
- Query history
- Response viewer
- Request/response timing
- Multiple tabs

```tsx
// pages/DevTools.tsx
import { Editor } from '@monaco-editor/react';
import { useState } from 'react';
import { executeLQL } from '../api/lexum';

export const DevTools: React.FC = () => {
  const [query, setQuery] = useState('FROM my_index | LIMIT 10');
  const [result, setResult] = useState(null);

  const handleExecute = async () => {
    const response = await executeLQL(query);
    setResult(response);
  };

  return (
    <div className="dev-tools">
      <div className="editor-panel">
        <Editor
          language="lql"
          value={query}
          onChange={setQuery}
          theme="vs-dark"
          options={{
            minimap: { enabled: false },
            fontSize: 14,
          }}
        />
        <button onClick={handleExecute}>Execute</button>
      </div>
      <div className="result-panel">
        <pre>{JSON.stringify(result, null, 2)}</pre>
      </div>
    </div>
  );
};
```

### 5. Index Management

Manage indices, mappings, and settings.

**Features:**
- List all indices
- Create new index
- Update settings
- Manage mappings
- Reindex data
- Delete indices
- Index statistics

```tsx
// pages/IndexManagement.tsx
import { DataGrid } from '@mui/x-data-grid';
import { useIndices } from '../hooks/useIndices';

export const IndexManagement: React.FC = () => {
  const { indices, loading } = useIndices();

  const columns = [
    { field: 'name', headerName: 'Name', width: 200 },
    { field: 'health', headerName: 'Health', width: 100 },
    { field: 'docs', headerName: 'Documents', width: 150 },
    { field: 'size', headerName: 'Size', width: 150 },
    { field: 'shards', headerName: 'Shards', width: 100 },
  ];

  return (
    <div className="index-management">
      <div className="toolbar">
        <button>Create Index</button>
        <button>Delete Selected</button>
      </div>
      <DataGrid
        rows={indices}
        columns={columns}
        loading={loading}
        checkboxSelection
      />
    </div>
  );
};
```

### 6. Monitoring & Observability

Real-time cluster monitoring.

**Metrics Displayed:**
- Cluster health
- Node status
- CPU usage
- Memory usage
- Disk usage
- Network I/O
- Request rate
- Search latency
- Index rate

```tsx
// pages/Monitoring.tsx
import { useRealTimeMetrics } from '../hooks/useRealTimeMetrics';
import { LineChart, Line, XAxis, YAxis, Tooltip } from 'recharts';

export const Monitoring: React.FC = () => {
  const metrics = useRealTimeMetrics();

  return (
    <div className="monitoring">
      <h2>Cluster Metrics</h2>
      
      <div className="metric-card">
        <h3>Request Rate</h3>
        <LineChart data={metrics.requestRate} width={600} height={300}>
          <XAxis dataKey="time" />
          <YAxis />
          <Tooltip />
          <Line type="monotone" dataKey="value" stroke="#8884d8" />
        </LineChart>
      </div>

      <div className="metric-card">
        <h3>Search Latency (p95)</h3>
        <LineChart data={metrics.latency} width={600} height={300}>
          <XAxis dataKey="time" />
          <YAxis />
          <Tooltip />
          <Line type="monotone" dataKey="p95" stroke="#82ca9d" />
        </LineChart>
      </div>
    </div>
  );
};
```

### 7. Logs

Centralized log viewer.

**Features:**
- Real-time log streaming
- Log level filtering
- Search logs
- Time range selection
- Log export
- Tail mode

```tsx
// pages/Logs.tsx
import { useLogStream } from '../hooks/useLogStream';
import { VirtualizedList } from '../components/VirtualizedList';

export const Logs: React.FC = () => {
  const { logs, filters, setFilters } = useLogStream();

  return (
    <div className="logs-page">
      <div className="log-filters">
        <select 
          value={filters.level} 
          onChange={(e) => setFilters({ ...filters, level: e.target.value })}
        >
          <option value="">All Levels</option>
          <option value="ERROR">Error</option>
          <option value="WARN">Warning</option>
          <option value="INFO">Info</option>
          <option value="DEBUG">Debug</option>
        </select>
        <input
          type="text"
          placeholder="Search logs..."
          value={filters.search}
          onChange={(e) => setFilters({ ...filters, search: e.target.value })}
        />
      </div>
      
      <VirtualizedList
        items={logs}
        renderItem={(log) => (
          <div className={`log-entry log-${log.level.toLowerCase()}`}>
            <span className="log-timestamp">{log.timestamp}</span>
            <span className="log-level">{log.level}</span>
            <span className="log-message">{log.message}</span>
          </div>
        )}
      />
    </div>
  );
};
```

### 8. Security Management

User and role administration.

**Features:**
- User management (CRUD)
- Role management
- Permission assignment
- API key generation
- Audit log

```tsx
// pages/Security.tsx
import { Tabs, Tab } from '@mui/material';
import { UserManagement } from '../components/UserManagement';
import { RoleManagement } from '../components/RoleManagement';
import { ApiKeys } from '../components/ApiKeys';

export const Security: React.FC = () => {
  const [tab, setTab] = useState(0);

  return (
    <div className="security-page">
      <Tabs value={tab} onChange={(_, v) => setTab(v)}>
        <Tab label="Users" />
        <Tab label="Roles" />
        <Tab label="API Keys" />
      </Tabs>
      
      {tab === 0 && <UserManagement />}
      {tab === 1 && <RoleManagement />}
      {tab === 2 && <ApiKeys />}
    </div>
  );
};
```

## Real-Time Updates

### WebSocket Connection

```typescript
// hooks/useWebSocket.ts
import { useEffect, useState } from 'react';
import { io, Socket } from 'socket.io-client';

export const useWebSocket = (url: string) => {
  const [socket, setSocket] = useState<Socket | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const s = io(url);
    
    s.on('connect', () => {
      setConnected(true);
      console.log('WebSocket connected');
    });
    
    s.on('disconnect', () => {
      setConnected(false);
      console.log('WebSocket disconnected');
    });
    
    setSocket(s);
    
    return () => {
      s.close();
    };
  }, [url]);

  return { socket, connected };
};
```

### Real-Time Metrics

```typescript
// hooks/useRealTimeMetrics.ts
import { useWebSocket } from './useWebSocket';
import { useState, useEffect } from 'react';

export const useRealTimeMetrics = () => {
  const { socket } = useWebSocket('http://localhost:9200');
  const [metrics, setMetrics] = useState({
    requestRate: [],
    latency: [],
    errors: [],
  });

  useEffect(() => {
    if (!socket) return;

    socket.on('metrics', (data) => {
      setMetrics((prev) => ({
        requestRate: [...prev.requestRate.slice(-100), data.requestRate],
        latency: [...prev.latency.slice(-100), data.latency],
        errors: [...prev.errors.slice(-100), data.errors],
      }));
    });

    return () => {
      socket.off('metrics');
    };
  }, [socket]);

  return metrics;
};
```

## API Client

```typescript
// api/lexum.ts
import axios from 'axios';

const client = axios.create({
  baseURL: 'http://localhost:9200',
  timeout: 30000,
});

// Add API key to requests
client.interceptors.request.use((config) => {
  const apiKey = localStorage.getItem('apiKey');
  if (apiKey) {
    config.headers['X-API-Key'] = apiKey;
  }
  return config;
});

export const executeLQL = async (query: string) => {
  const response = await client.post('/_lql', { query });
  return response.data;
};

export const getIndices = async () => {
  const response = await client.get('/_cat/indices');
  return response.data;
};

export const getClusterHealth = async () => {
  const response = await client.get('/_cluster/health');
  return response.data;
};

export const searchIndex = async (index: string, query: any) => {
  const response = await client.post(`/${index}/_search`, query);
  return response.data;
};
```

## Theming

```typescript
// theme.ts
import { createTheme } from '@mui/material/styles';

export const lightTheme = createTheme({
  palette: {
    mode: 'light',
    primary: {
      main: '#1976d2',
    },
    secondary: {
      main: '#dc004e',
    },
  },
});

export const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#90caf9',
    },
    secondary: {
      main: '#f48fb1',
    },
  },
});
```

```tsx
// App.tsx
import { ThemeProvider } from '@mui/material/styles';
import { useState } from 'react';
import { lightTheme, darkTheme } from './theme';

export const App: React.FC = () => {
  const [isDark, setIsDark] = useState(false);

  return (
    <ThemeProvider theme={isDark ? darkTheme : lightTheme}>
      <CssBaseline />
      <AppLayout onThemeToggle={() => setIsDark(!isDark)} />
    </ThemeProvider>
  );
};
```

## Configuration

```json
// config.json
{
  "defaultConnection": "http://localhost:9200",
  "savedConnections": [
    {
      "name": "Local",
      "url": "http://localhost:9200",
      "apiKey": ""
    },
    {
      "name": "Production",
      "url": "https://search.example.com",
      "apiKey": "..."
    }
  ],
  "preferences": {
    "theme": "dark",
    "defaultIndex": "logs-*",
    "pageSize": 50,
    "refreshInterval": 5000
  }
}
```

## Building

### Development

```bash
npm run dev
```

### Production Build

```bash
# Build for current platform
npm run build

# Package
npm run package
```

### Multi-Platform Build

```bash
# macOS
npm run package:mac

# Windows
npm run package:win

# Linux
npm run package:linux

# All platforms
npm run package:all
```

### Electron Builder Config

```json
// electron-builder.json
{
  "appId": "com.lexum.gui",
  "productName": "Lexum",
  "directories": {
    "output": "dist"
  },
  "files": [
    "build/**/*",
    "node_modules/**/*"
  ],
  "mac": {
    "target": ["dmg", "zip"],
    "category": "public.app-category.developer-tools"
  },
  "win": {
    "target": ["nsis", "portable"]
  },
  "linux": {
    "target": ["AppImage", "deb", "rpm"],
    "category": "Development"
  }
}
```

## Auto-Update

```typescript
// main/updater.ts
import { autoUpdater } from 'electron-updater';

autoUpdater.checkForUpdatesAndNotify();

autoUpdater.on('update-available', () => {
  console.log('Update available');
});

autoUpdater.on('update-downloaded', () => {
  autoUpdater.quitAndInstall();
});
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + K` | Open command palette |
| `Cmd/Ctrl + /` | Toggle sidebar |
| `Cmd/Ctrl + Enter` | Execute query |
| `Cmd/Ctrl + S` | Save current view |
| `Cmd/Ctrl + F` | Search |
| `Cmd/Ctrl + ,` | Open settings |
| `F5` | Refresh |

## Performance Optimization

1. **Virtual Scrolling**: For large result sets
2. **Code Splitting**: Lazy load routes
3. **Memoization**: Cache expensive computations
4. **WebWorkers**: Offload heavy processing
5. **Debouncing**: Reduce API calls

## Testing

```bash
# Unit tests
npm run test

# E2E tests
npm run test:e2e

# Coverage
npm run test:coverage
```

## See Also

- [API Reference](./API_REFERENCE.md)
- [Development](./DEVELOPMENT.md)
- [Architecture](./ARCHITECTURE.md)

