# Snapshot Repository Configuration

This document describes how to configure snapshot repositories in Lexum for backing up and restoring your search indices.

## Overview

Snapshot repositories in Lexum allow you to:
- Create point-in-time backups of your indices
- Restore indices from snapshots
- Manage snapshot retention policies
- Support multiple storage backends (filesystem, S3, GCS, Azure)

## Configuration Structure

### Global Snapshot Settings

```yaml
snapshots:
  path: "./snapshots"              # Global snapshot storage path
  max_snapshots: 100               # Maximum snapshots across all repositories
  compression_enabled: true        # Enable compression for snapshots
  repositories:                    # List of configured repositories
    - name: "repo1"
      repository_type: "fs"
      settings: { ... }
```

### Repository Configuration

Each repository has the following structure:

```yaml
- name: "repository_name"          # Unique repository identifier
  repository_type: "fs"            # Storage type: fs, s3, gcs, azure
  settings:                        # Repository-specific settings
    location: "path_or_bucket"     # Storage location
    compress: true                 # Enable compression
    chunk_size: "1gb"              # Chunk size for snapshots
    max_restore_bytes_per_sec: "40mb"    # Restore rate limit
    max_snapshot_bytes_per_sec: "40mb"   # Snapshot rate limit
    readonly: false                # Read-only repository
    max_snapshots: 1000            # Max snapshots in this repository
    retention_policy:              # Snapshot retention rules
      keep_for_days: 30
      keep_count: 10
      delete_after_days: 90
```

## Repository Types

### Filesystem Repository (fs)

Store snapshots on the local filesystem or mounted storage.

```yaml
- name: "local_repo"
  repository_type: "fs"
  settings:
    location: "/var/lib/lexum/snapshots"
    compress: true
    chunk_size: "1gb"
    max_restore_bytes_per_sec: "40mb"
    max_snapshot_bytes_per_sec: "40mb"
    readonly: false
    max_snapshots: 1000
    retention_policy:
      keep_for_days: 30
      keep_count: 10
      delete_after_days: 90
```

**Requirements:**
- Valid filesystem path
- Write permissions for the Lexum process
- Sufficient disk space

### AWS S3 Repository (s3)

Store snapshots in Amazon S3 or S3-compatible services.

```yaml
- name: "s3_repo"
  repository_type: "s3"
  settings:
    location: "my-lexum-snapshots"  # S3 bucket name
    compress: true
    chunk_size: "1gb"
    max_restore_bytes_per_sec: "40mb"
    max_snapshot_bytes_per_sec: "40mb"
    readonly: false
    max_snapshots: 5000
    retention_policy:
      keep_for_days: 60
      keep_count: 50
      delete_after_days: 180
    s3_settings:
      region: "us-east-1"
      access_key_id: "${AWS_ACCESS_KEY_ID}"
      secret_access_key: "${AWS_SECRET_ACCESS_KEY}"
      endpoint: null  # Use default AWS S3 endpoint
      path_style: false
      server_side_encryption: "AES256"
```

**S3 Settings:**
- `region`: AWS region (required)
- `access_key_id`: AWS access key (optional, can use IAM roles)
- `secret_access_key`: AWS secret key (optional, can use IAM roles)
- `endpoint`: Custom S3 endpoint for S3-compatible services
- `path_style`: Use path-style URLs instead of virtual-hosted-style
- `server_side_encryption`: Encryption algorithm (AES256, aws:kms)

**Authentication Methods:**
1. Environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`
2. IAM roles (recommended for EC2 instances)
3. AWS credentials file (`~/.aws/credentials`)
4. Explicit configuration in YAML

### Google Cloud Storage Repository (gcs)

Store snapshots in Google Cloud Storage.

```yaml
- name: "gcs_repo"
  repository_type: "gcs"
  settings:
    location: "my-lexum-snapshots"  # GCS bucket name
    compress: true
    chunk_size: "1gb"
    max_restore_bytes_per_sec: "40mb"
    max_snapshot_bytes_per_sec: "40mb"
    readonly: false
    max_snapshots: 5000
    retention_policy:
      keep_for_days: 60
      keep_count: 50
      delete_after_days: 180
    gcs_settings:
      project_id: "my-gcp-project"
      service_account_key_file: "/path/to/service-account-key.json"
      # OR use application credentials
      # application_credentials: "/path/to/application-credentials.json"
```

**GCS Settings:**
- `project_id`: GCP project ID (optional)
- `service_account_key_file`: Path to service account JSON key file
- `application_credentials`: Path to application credentials JSON file

**Authentication Methods:**
1. Service account key file
2. Application credentials file
3. Default credentials (gcloud auth)
4. Environment variable: `GOOGLE_APPLICATION_CREDENTIALS`

### Azure Blob Storage Repository (azure)

Store snapshots in Azure Blob Storage.

```yaml
- name: "azure_repo"
  repository_type: "azure"
  settings:
    location: "lexum-snapshots"  # Azure container name
    compress: true
    chunk_size: "1gb"
    max_restore_bytes_per_sec: "40mb"
    max_snapshot_bytes_per_sec: "40mb"
    readonly: false
    max_snapshots: 5000
    retention_policy:
      keep_for_days: 60
      keep_count: 50
      delete_after_days: 180
    azure_settings:
      account_name: "mystorageaccount"
      account_key: "${AZURE_STORAGE_ACCOUNT_KEY}"
      # OR use connection string
      # connection_string: "${AZURE_STORAGE_CONNECTION_STRING}"
      container_name: "snapshots"
```

**Azure Settings:**
- `account_name`: Azure storage account name
- `account_key`: Azure storage account key
- `connection_string`: Azure storage connection string (alternative to account_name/key)
- `container_name`: Azure blob container name

**Authentication Methods:**
1. Account name and key
2. Connection string
3. Environment variables: `AZURE_STORAGE_ACCOUNT_NAME`, `AZURE_STORAGE_ACCOUNT_KEY`
4. Managed identity (for Azure VMs)

## Retention Policies

Retention policies control how long snapshots are kept and when they are automatically deleted.

```yaml
retention_policy:
  keep_for_days: 30        # Keep snapshots for 30 days
  keep_count: 10           # Always keep at least 10 snapshots
  delete_after_days: 90    # Delete snapshots older than 90 days
```

**Policy Rules:**
- At least one retention rule must be specified
- `keep_for_days`: Keep snapshots for this many days (1-3650)
- `keep_count`: Always keep this many snapshots (1-10000)
- `delete_after_days`: Delete snapshots older than this many days (1-3650)
- `keep_for_days` cannot be greater than `delete_after_days`

## Size Configuration

Size values can be specified in various formats:

```yaml
chunk_size: "1gb"                    # 1 gigabyte
chunk_size: "512mb"                  # 512 megabytes
chunk_size: "1024kb"                 # 1024 kilobytes
chunk_size: "1b"                     # 1 byte

max_restore_bytes_per_sec: "40mb"    # 40 megabytes per second
max_snapshot_bytes_per_sec: "40mb"   # 40 megabytes per second
```

**Supported Units:**
- `b` - bytes
- `kb` - kilobytes
- `mb` - megabytes
- `gb` - gigabytes
- `tb` - terabytes

## Validation Rules

The configuration system validates all settings to ensure they are correct:

### Repository Name Validation
- Cannot be empty
- Must contain only alphanumeric characters, hyphens, and underscores
- Must be unique across all repositories

### Repository Type Validation
- Must be one of: `fs`, `s3`, `gcs`, `azure`

### Location Validation
- Filesystem: Must be a valid, non-empty path
- S3/GCS/Azure: Must be a valid bucket/container name

### Size Validation
- Must be in format like "1gb", "512mb", "1024kb"
- Must have at least one digit
- Must end with a valid unit

### Retention Policy Validation
- At least one rule must be specified
- Days must be between 1 and 3650
- Count must be between 1 and 10000
- `keep_for_days` cannot exceed `delete_after_days`

### Cloud Provider Settings Validation
- S3: Valid AWS region format
- GCS: At least one authentication method must be configured
- Azure: At least one authentication method must be configured

## Environment Variable Overrides

You can override any configuration value using environment variables:

```bash
# Global settings
export LEXUM_SNAPSHOTS_PATH="/custom/snapshots"
export LEXUM_SNAPSHOTS_MAX_SNAPSHOTS=200
export LEXUM_SNAPSHOTS_COMPRESSION_ENABLED=true

# Repository settings (for first repository)
export LEXUM_SNAPSHOTS_REPOSITORIES_0_NAME="my_repo"
export LEXUM_SNAPSHOTS_REPOSITORIES_0_REPOSITORY_TYPE="s3"
export LEXUM_SNAPSHOTS_REPOSITORIES_0_SETTINGS_LOCATION="my-bucket"

# S3 settings
export LEXUM_SNAPSHOTS_REPOSITORIES_0_SETTINGS_S3_SETTINGS_REGION="us-west-2"
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
```

## Example Configurations

### Development Setup
```yaml
snapshots:
  path: "./snapshots"
  max_snapshots: 10
  compression_enabled: true
  repositories:
    - name: "dev_repo"
      repository_type: "fs"
      settings:
        location: "./snapshots/dev"
        compress: true
        chunk_size: "100mb"
        max_restore_bytes_per_sec: "10mb"
        max_snapshot_bytes_per_sec: "10mb"
        readonly: false
        max_snapshots: 10
        retention_policy:
          keep_count: 5
          delete_after_days: 7
```

### Production Setup
```yaml
snapshots:
  path: "/var/lib/lexum/snapshots"
  max_snapshots: 1000
  compression_enabled: true
  repositories:
    - name: "local_backup"
      repository_type: "fs"
      settings:
        location: "/var/lib/lexum/snapshots/local"
        compress: true
        chunk_size: "1gb"
        max_restore_bytes_per_sec: "100mb"
        max_snapshot_bytes_per_sec: "100mb"
        readonly: false
        max_snapshots: 100
        retention_policy:
          keep_for_days: 7
          keep_count: 10
          delete_after_days: 30
    
    - name: "s3_backup"
      repository_type: "s3"
      settings:
        location: "lexum-prod-snapshots"
        compress: true
        chunk_size: "1gb"
        max_restore_bytes_per_sec: "200mb"
        max_snapshot_bytes_per_sec: "200mb"
        readonly: false
        max_snapshots: 1000
        retention_policy:
          keep_for_days: 30
          keep_count: 50
          delete_after_days: 365
        s3_settings:
          region: "us-east-1"
          server_side_encryption: "AES256"
```

## Best Practices

1. **Use multiple repositories**: Keep local snapshots for quick recovery and cloud snapshots for disaster recovery
2. **Configure retention policies**: Set appropriate retention based on your recovery requirements
3. **Use compression**: Enable compression to save storage space
4. **Set rate limits**: Configure appropriate rate limits to avoid impacting production performance
5. **Use environment variables**: Store sensitive credentials in environment variables, not in configuration files
6. **Test restores**: Regularly test snapshot restoration to ensure backups are working
7. **Monitor storage**: Keep track of snapshot storage usage and costs
8. **Use IAM roles**: Prefer IAM roles over access keys for cloud storage when possible

## Troubleshooting

### Common Issues

1. **Repository validation errors**: Check that all required fields are present and valid
2. **Authentication failures**: Verify cloud provider credentials and permissions
3. **Permission errors**: Ensure the Lexum process has appropriate read/write permissions
4. **Storage quota exceeded**: Check cloud storage quotas and local disk space
5. **Network connectivity**: Verify network connectivity to cloud storage services

### Debug Configuration

Enable debug logging to troubleshoot configuration issues:

```yaml
logging:
  level: "debug"
  format: "pretty"
  outputs:
    - "stdout"
```

This will show detailed information about configuration loading and validation.