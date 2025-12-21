---
sidebar_position: 2
title: Event Pipeline
description: Understanding the high-performance event processing pipeline
---

# Event Pipeline

The event pipeline is the core of EventFlux's high-performance processing. It implements a lock-free, zero-allocation architecture designed for throughput exceeding 1 million events per second.

## Pipeline Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                       Event Pipeline                              │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────┐    ┌─────────────────────────────────┐    ┌──────┐ │
│  │ Source  │───▶│      Processor Chain             │───▶│ Sink │ │
│  │ Handler │    │                                  │    │      │ │
│  └─────────┘    │  Filter → Project → Window → Agg │    └──────┘ │
│       │         └─────────────────────────────────┘        │     │
│       │                        │                           │     │
│       └────────────────────────┴───────────────────────────┘     │
│                                │                                  │
│                    ┌───────────▼───────────┐                     │
│                    │    State Manager      │                     │
│                    │  (Checkpoint/Recovery)│                     │
│                    └───────────────────────┘                     │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

## Event Flow

### 1. Source Ingestion

Events enter the pipeline through source handlers:

```rust
pub trait SourceHandler: Send + Sync {
    /// Receive events from external source
    fn receive(&mut self) -> Option<StreamEvent>;

    /// Get source metadata
    fn metadata(&self) -> SourceMetadata;
}

// Example: Stream source
pub struct StreamSource {
    stream_name: String,
    input_queue: ArrayQueue<StreamEvent>,
}

impl SourceHandler for StreamSource {
    fn receive(&mut self) -> Option<StreamEvent> {
        self.input_queue.pop()
    }
}
```

### 2. Processor Chain

Events flow through a chain of processors:

```rust
pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChain {
    pub fn process(&mut self, event: StreamEvent) -> Vec<StreamEvent> {
        let mut events = vec![event];

        for processor in &mut self.processors {
            let mut next_events = Vec::new();
            for e in events {
                next_events.extend(processor.process(e));
            }
            events = next_events;
        }

        events
    }
}
```

### 3. Sink Output

Processed events are sent to sinks:

```rust
pub trait SinkHandler: Send + Sync {
    /// Send event to output
    fn send(&mut self, event: StreamEvent) -> Result<(), SinkError>;

    /// Flush buffered events
    fn flush(&mut self) -> Result<(), SinkError>;
}
```

## Lock-Free Queues

EventFlux uses crossbeam's `ArrayQueue` for lock-free event passing:

```rust
use crossbeam::queue::ArrayQueue;

pub struct EventQueue {
    queue: ArrayQueue<StreamEvent>,
    capacity: usize,
}

impl EventQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
            capacity,
        }
    }

    /// Non-blocking push
    pub fn try_push(&self, event: StreamEvent) -> Result<(), StreamEvent> {
        self.queue.push(event)
    }

    /// Non-blocking pop
    pub fn try_pop(&self) -> Option<StreamEvent> {
        self.queue.pop()
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.queue.is_full()
    }

    /// Current queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
```

### Benefits of Lock-Free Design

| Aspect | Lock-Free | Traditional Locks |
|--------|-----------|-------------------|
| Contention | Minimal | Can cause blocking |
| Latency | Predictable | Variable |
| Throughput | High | Depends on contention |
| Complexity | Higher | Simpler |

## Object Pooling

Pre-allocated objects minimize allocation overhead:

```rust
pub struct EventPool {
    pool: ArrayQueue<Box<StreamEventData>>,
    default_capacity: usize,
}

impl EventPool {
    pub fn new(capacity: usize) -> Self {
        let pool = ArrayQueue::new(capacity);
        // Pre-allocate events
        for _ in 0..capacity {
            let _ = pool.push(Box::new(StreamEventData::default()));
        }
        Self {
            pool,
            default_capacity: capacity,
        }
    }

    /// Acquire event from pool (or allocate new)
    pub fn acquire(&self) -> Box<StreamEventData> {
        self.pool.pop().unwrap_or_else(|| Box::new(StreamEventData::default()))
    }

    /// Return event to pool
    pub fn release(&self, event: Box<StreamEventData>) {
        // Clear and return to pool
        let _ = self.pool.push(event);
    }
}
```

## Backpressure Management

### Strategy Configuration

```rust
pub enum BackpressureStrategy {
    /// Block until space available
    Block,
    /// Drop oldest events
    DropOldest,
    /// Drop newest events
    DropNewest,
    /// Grow unbounded (use with caution)
    Unbounded,
}

pub struct PipelineConfig {
    pub buffer_size: usize,
    pub backpressure: BackpressureStrategy,
    pub metrics_enabled: bool,
}
```

### Implementation

```rust
impl EventQueue {
    pub fn push_with_strategy(
        &self,
        event: StreamEvent,
        strategy: &BackpressureStrategy,
    ) -> Result<(), PushError> {
        match strategy {
            BackpressureStrategy::Block => {
                // Spin until space available
                while self.queue.push(event.clone()).is_err() {
                    std::hint::spin_loop();
                }
                Ok(())
            }
            BackpressureStrategy::DropOldest => {
                if self.queue.is_full() {
                    let _ = self.queue.pop(); // Drop oldest
                }
                self.queue.push(event).map_err(|_| PushError::Full)
            }
            BackpressureStrategy::DropNewest => {
                self.queue.push(event).map_err(|_| PushError::Full)
            }
            BackpressureStrategy::Unbounded => {
                // This would require a different queue type
                unimplemented!()
            }
        }
    }
}
```

## Pipeline Metrics

### Performance Counters

```rust
pub struct PipelineMetrics {
    /// Events processed per second
    pub throughput: AtomicU64,
    /// Current queue depth
    pub queue_depth: AtomicUsize,
    /// Events dropped due to backpressure
    pub dropped_events: AtomicU64,
    /// Processing latency (nanoseconds)
    pub latency_ns: AtomicU64,
}

impl PipelineMetrics {
    pub fn record_event(&self, latency_ns: u64) {
        self.throughput.fetch_add(1, Ordering::Relaxed);
        // Update latency using exponential moving average
        let current = self.latency_ns.load(Ordering::Relaxed);
        let new = (current * 9 + latency_ns) / 10;
        self.latency_ns.store(new, Ordering::Relaxed);
    }
}
```

## Multi-Stage Processing

### Parallel Stages

```rust
pub struct ParallelPipeline {
    stages: Vec<ProcessingStage>,
    worker_threads: usize,
}

pub struct ProcessingStage {
    input: ArrayQueue<StreamEvent>,
    output: ArrayQueue<StreamEvent>,
    processor: Box<dyn Processor>,
}

impl ParallelPipeline {
    pub fn run(&self) {
        // Each stage runs in its own thread
        for stage in &self.stages {
            std::thread::spawn(move || {
                loop {
                    if let Some(event) = stage.input.pop() {
                        let results = stage.processor.process(event);
                        for result in results {
                            let _ = stage.output.push(result);
                        }
                    }
                }
            });
        }
    }
}
```

## Best Practices

:::tip Pipeline Optimization

1. **Size queues appropriately** - Match queue size to expected burst capacity
2. **Choose backpressure wisely** - Block for guaranteed delivery, drop for real-time
3. **Monitor queue depths** - High depths indicate bottlenecks
4. **Profile hot paths** - Identify and optimize slow processors

:::

:::caution Performance Pitfalls

- **Avoid allocations in hot paths** - Use object pools
- **Minimize lock contention** - Use lock-free structures
- **Batch when possible** - Reduce per-event overhead
- **Watch for backpressure** - It can cascade through the pipeline

:::

## Next Steps

- [State Management](/docs/architecture/state-management) - Checkpointing and recovery
- [Architecture Overview](/docs/architecture/overview) - System architecture
- [Rust API](/docs/rust-api/getting-started) - Programmatic usage
