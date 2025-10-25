## Why

Administrators need advanced operations for managing production clusters including snapshots, index templates, reindexing, cluster settings, and task management. These operations are essential for maintaining healthy production deployments.

## What Changes

- Implement snapshot and restore operations
- Add index templates for automatic index creation
- Implement index aliases
- Add reindexing operations
- Implement cluster settings API
- Add node stats and info APIs
- Implement task management API
- Add index rollover
- Implement index lifecycle management (ILM)

## Impact

- Affected specs: `admin-operations`, `snapshot-restore`, `index-templates`
- Affected code: Creates `lexum-server/src/api/admin/`:
  - `snapshot.rs` - Snapshot operations
  - `template.rs` - Index templates
  - `reindex.rs` - Reindexing
  - `settings.rs` - Cluster settings
  - `tasks.rs` - Task management
- Requires admin role permissions

