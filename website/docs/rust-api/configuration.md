---
sidebar_position: 2
title: Configuration
description: Configure EventFlux runtime behavior
---

# Configuration

This guide covers all configuration options for customizing EventFlux runtime behavior.

## Runtime Configuration

### Basic Configuration

```rust
use eventflux::config::RuntimeConfig;
use std::time::Duration;

let config = RuntimeConfig::builder()
    .buffer_size(10_000)
    .backpressure_strategy(BackpressureStrategy::Block)
    .checkpoint_interval(Duration::from_secs(60))
    .build();

let runtime = manager.create_runtime_with_config(app_definition, config)?;
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `buffer_size` | `usize` | `8192` | Event queue capacity |
| `backpressure_strategy` | `Strategy` | `Block` | How to handle full queues |
| `checkpoint_interval` | `Duration` | `60s` | State checkpoint frequency |
| `parallelism` | `usize` | `num_cpus` | Processing threads |

## Backpressure Strategies

### Block (Default)

Blocks the sender until space is available:

```rust
let config = RuntimeConfig::builder()
    .backpressure_strategy(BackpressureStrategy::Block)
    .build();
```

**Use when:** Guaranteed delivery is required, sender can tolerate blocking.

### Drop Oldest

Drops the oldest events when the buffer is full:

```rust
let config = RuntimeConfig::builder()
    .backpressure_strategy(BackpressureStrategy::DropOldest)
    .build();
```

**Use when:** Latest data is more important than historical data.

### Drop Newest

Rejects new events when the buffer is full:

```rust
let config = RuntimeConfig::builder()
    .backpressure_strategy(BackpressureStrategy::DropNewest)
    .build();
```

**Use when:** Historical data must be preserved.

## Buffer Sizing

### Calculating Buffer Size

Consider:
- **Burst capacity**: Maximum events expected in a burst
- **Memory constraints**: Each event consumes memory
- **Latency tolerance**: Larger buffers = higher latency potential

```rust
// For high-throughput scenarios
let config = RuntimeConfig::builder()
    .buffer_size(100_000)  // Handle large bursts
    .build();

// For low-latency scenarios
let config = RuntimeConfig::builder()
    .buffer_size(1_000)    // Keep buffers small
    .build();
```

### Memory Estimation

```rust
// Approximate memory usage
let event_size_bytes = 200;  // Average event size
let buffer_size = 10_000;
let estimated_memory = event_size_bytes * buffer_size;
// ~2MB for this configuration
```

## Checkpointing Configuration

### Checkpoint Interval

```rust
use std::time::Duration;

let config = RuntimeConfig::builder()
    .checkpoint_interval(Duration::from_secs(30))  // Every 30 seconds
    .build();
```

### Checkpoint Storage

```rust
use eventflux::config::CheckpointConfig;
use std::path::PathBuf;

let checkpoint_config = CheckpointConfig {
    enabled: true,
    interval: Duration::from_secs(60),
    compression: true,
    storage: StorageConfig::Local {
        path: PathBuf::from("/var/eventflux/checkpoints"),
    },
    max_retained: 10,
};

let config = RuntimeConfig::builder()
    .checkpoint_config(checkpoint_config)
    .build();
```

### Redis Storage Backend

```rust
let checkpoint_config = CheckpointConfig {
    enabled: true,
    interval: Duration::from_secs(60),
    compression: true,
    storage: StorageConfig::Redis {
        url: "redis://localhost:6379".to_string(),
        prefix: "eventflux:checkpoints".to_string(),
        ttl_seconds: 86400,  // 24 hours
    },
    max_retained: 10,
};
```

## Parallelism

### Thread Configuration

```rust
let config = RuntimeConfig::builder()
    .parallelism(4)  // Use 4 processing threads
    .build();
```

### Auto-Detection

```rust
let config = RuntimeConfig::builder()
    .parallelism(num_cpus::get())  // Use all available CPUs
    .build();
```

## Complete Configuration Example

```rust
use eventflux::prelude::*;
use eventflux::config::*;
use std::time::Duration;
use std::path::PathBuf;

fn create_production_config() -> RuntimeConfig {
    RuntimeConfig::builder()
        // Event processing
        .buffer_size(50_000)
        .backpressure_strategy(BackpressureStrategy::Block)
        .parallelism(num_cpus::get())

        // Checkpointing
        .checkpoint_config(CheckpointConfig {
            enabled: true,
            interval: Duration::from_secs(30),
            compression: true,
            storage: StorageConfig::Local {
                path: PathBuf::from("/var/eventflux/checkpoints"),
            },
            max_retained: 20,
        })

        // Metrics
        .metrics_enabled(true)
        .metrics_interval(Duration::from_secs(10))

        .build()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = EventFluxManager::new();
    let config = create_production_config();

    let app = r#"
        DEFINE STREAM Input (value INT);
        SELECT value FROM Input INSERT INTO Output;
    "#;

    let runtime = manager.create_runtime_with_config(app, config)?;
    runtime.start();

    Ok(())
}
```

## Environment Variables

Some configuration can be set via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `EVENTFLUX_BUFFER_SIZE` | Event buffer size | `8192` |
| `EVENTFLUX_PARALLELISM` | Processing threads | CPU count |
| `EVENTFLUX_CHECKPOINT_DIR` | Checkpoint directory | `/tmp/eventflux` |
| `EVENTFLUX_LOG_LEVEL` | Log verbosity | `info` |

```rust
use std::env;

// Override from environment
let buffer_size: usize = env::var("EVENTFLUX_BUFFER_SIZE")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(8192);

let config = RuntimeConfig::builder()
    .buffer_size(buffer_size)
    .build();
```

## Configuration Profiles

### Development Profile

```rust
fn dev_config() -> RuntimeConfig {
    RuntimeConfig::builder()
        .buffer_size(1_000)
        .checkpoint_config(CheckpointConfig {
            enabled: false,  // Disable for faster iteration
            ..Default::default()
        })
        .build()
}
```

### Production Profile

```rust
fn prod_config() -> RuntimeConfig {
    RuntimeConfig::builder()
        .buffer_size(50_000)
        .backpressure_strategy(BackpressureStrategy::Block)
        .checkpoint_config(CheckpointConfig {
            enabled: true,
            interval: Duration::from_secs(30),
            compression: true,
            storage: StorageConfig::Redis {
                url: env::var("REDIS_URL").unwrap(),
                prefix: "eventflux:prod".to_string(),
                ttl_seconds: 86400 * 7,  // 7 days
            },
            max_retained: 50,
        })
        .metrics_enabled(true)
        .build()
}
```

### Testing Profile

```rust
fn test_config() -> RuntimeConfig {
    RuntimeConfig::builder()
        .buffer_size(100)  // Small for predictable tests
        .parallelism(1)    // Single-threaded for determinism
        .checkpoint_config(CheckpointConfig {
            enabled: false,
            ..Default::default()
        })
        .build()
}
```

## Best Practices

:::tip Configuration Guidelines

1. **Start with defaults** - Only customize what you need
2. **Profile your workload** - Measure before tuning
3. **Test configuration changes** - Verify performance impact
4. **Use environment variables** - For deployment flexibility
5. **Document your choices** - Explain non-default settings

:::

:::caution Common Pitfalls

- **Buffer too small** - Causes excessive backpressure
- **Buffer too large** - Wastes memory, increases latency
- **Checkpointing too frequent** - Performance overhead
- **Checkpointing too rare** - Data loss risk on failure

:::

## Next Steps

- [Testing](/docs/rust-api/testing) - Test your configuration
- [Architecture](/docs/architecture/event-pipeline) - Understand the pipeline
- [State Management](/docs/architecture/state-management) - Checkpointing details
