# EventFlux

**Lightweight** SQL streaming for teams that don't need Flink.

EventFlux is a lightweight CEP (Complex Event Processing) engine that processes 100k+ events/sec in a single Docker container. No JVM, no Kubernetes, no ops team required.

**Lightweight means**:
- 50-100MB binary (vs 4GB+ JVM)
- Starts in milliseconds (vs 30+ seconds)
- Runs on a $50/month VPS
- Zero external dependencies

## The Problem

You need to:
- Detect patterns in event streams
- Aggregate metrics in real-time
- Join streams with reference data
- React to conditions within time windows

Your options are:
- **Flink**: Requires Kubernetes, 4GB+ JVM heap, ops expertise
- **Kafka Streams**: Needs Kafka cluster, Java expertise
- **Build it yourself**: 6+ months of work

For 100k events/sec, these are overkill.

## The Solution

EventFlux runs as a single binary:

```bash
docker run -v ./app.sql:/app.sql eventflux/engine /app.sql
```

That's it. No cluster management, no JVM tuning, no YAML manifests.

```
Event Sources (100k+ eps)
         |
    EventFlux
    - Pattern detection
    - Windows & aggregations
    - Stream-table joins
         |
    Sinks (Kafka, HTTP, DB)
```

## Why EventFlux

**Truly Lightweight**
- Single 50-100MB binary (not a 4GB JVM)
- Runs on a $50/month VPS (not a Kubernetes cluster)
- Starts in milliseconds (not 30+ seconds)
- Zero runtime dependencies

**SQL Interface**
- Standard SQL with streaming extensions
- No Java/Scala required
- Every developer knows SQL

**Predictable Performance**
- No GC pauses ever
- Deterministic memory usage
- Sub-millisecond latency for pattern detection

**Right-Sized for Most Use Cases**
- 100k+ events/sec on single node
- Perfect for IoT, analytics, telemetry
- Graduate to Flink when you actually need it

## Quick Example

```sql
-- Define input stream
CREATE STREAM StockTrades (
    symbol STRING,
    price DOUBLE,
    quantity INT,
    timestamp BIGINT
) WITH (
    'type' = 'source',
    'extension' = 'kafka',
    'kafka.brokers' = 'localhost:9092',
    'kafka.topic' = 'trades',
    'format' = 'json'
);

-- Detect price spikes: >5% increase within 1 minute
CREATE STREAM PriceSpikes AS
SELECT
    symbol,
    first(price) as start_price,
    last(price) as end_price,
    ((last(price) - first(price)) / first(price)) * 100 as percent_change
FROM StockTrades
    WINDOW TUMBLING (SIZE 1 MINUTE)
GROUP BY symbol
HAVING ((last(price) - first(price)) / first(price)) > 0.05;

-- Output to alert system
INSERT INTO Alerts
SELECT symbol, percent_change, 'PRICE_SPIKE' as alert_type
FROM PriceSpikes;
```

Compare to Flink:

```java
DataStream<Event> result = events
    .keyBy(e -> e.getSymbol())
    .window(TumblingEventTimeWindows.of(Time.minutes(1)))
    .aggregate(new PriceSpikeAggregator())
    .filter(spike -> spike.getPercentChange() > 0.05);
```

## When to Use EventFlux

**Good fit:**
- IoT backends (10-50k eps)
- E-commerce event tracking
- Internal analytics pipelines
- SaaS telemetry
- Prototyping before Flink

**Not a fit:**
- >500k eps sustained (consider Flink)
- Existing JVM/Kafka infrastructure
- Need 100+ connectors
- Require battle-tested at massive scale

## Capabilities

### Stream Processing

- **Windows**: tumbling, sliding, session, length, time-based
- **Joins**: stream-stream, stream-table, inner/outer
- **Aggregations**: sum, avg, count, min, max, stddev with group by
- **Partitioning**: parallel processing by key

### Pattern Detection

- Sequence matching with temporal constraints
- Logical operators (and, or, not)
- Count quantifiers (A{3}, A{2,5})

### State Management

- Incremental checkpointing with WAL
- Point-in-time recovery
- Redis backend for persistence
- 90-95% compression for snapshots

### Connectivity

- **Sources**: Kafka, HTTP, file
- **Sinks**: Kafka, HTTP, database
- **Tables**: PostgreSQL, MySQL, in-memory cache

### Performance

- Throughput: 100k-1M events/sec (single node)
- Latency: <10ms for pattern detection
- Memory: Lock-free crossbeam pipeline
- Startup: <100ms

## Getting Started

### Prerequisites

- Rust 1.85+ (or use Docker)
- Protocol Buffer Compiler (for gRPC features)

MSRV is enforced via `Cargo.toml` (`package.rust-version`) and CI; if you need to avoid installing Rust locally, use the
official Docker image instead.

### Docker (Recommended)

```bash
# Pull and run
docker run -v ./app.sql:/app.sql eventflux/engine /app.sql

# With configuration
docker run \
  -v ./app.sql:/app.sql \
  -v ./config.toml:/config.toml \
  eventflux/engine /app.sql --config /config.toml
```

### Build from Source

```bash
git clone https://github.com/eventflux-io/engine.git
cd engine
cargo build --release

# Run
./target/release/run_eventflux app.sql
```

### Configuration

EventFlux uses TOML configuration files and SQL WITH clauses:

```toml
# config.toml
[eventflux.application]
name = "my-app"

[eventflux.state]
backend = "redis"
redis_url = "redis://localhost:6379"
```

```sql
-- SQL WITH for stream-level config
CREATE STREAM Input (id INT, value STRING) WITH (
    'type' = 'source',
    'extension' = 'kafka',
    'kafka.brokers' = 'localhost:9092'
);
```

## Documentation

| Document | Description |
|----------|-------------|
| [DEV_GUIDE.md](DEV_GUIDE.md) | Building, testing, contributing |
| [ROADMAP.md](ROADMAP.md) | Implementation priorities |
| [MILESTONES.md](MILESTONES.md) | Release timeline |
| [feat/configuration/](feat/configuration/) | Configuration reference |

## Current Status

EventFlux is in active development. Core CEP functionality is implemented with 1,400+ passing tests.

**Implemented:**
- Lightweight single-binary deployment
- SQL parser with streaming extensions
- Window processors (9 types)
- Join processors (stream-stream, stream-table)
- Pattern and sequence matching
- Aggregations with group by
- State persistence with Redis
- TOML configuration system

**In Progress:**
- Source/sink connectors (Kafka, HTTP)
- Developer experience improvements
- Production hardening

**Planned:**
- CASE expressions
- Prometheus metrics
- Additional connectors

See [ROADMAP.md](ROADMAP.md) for detailed status.

## Comparison with Alternatives

| Feature | EventFlux | Flink | Kafka Streams |
|---------|-----------|-------|---------------|
| Deployment | Single binary | Kubernetes cluster | Kafka cluster |
| Memory | 50-100MB | 4GB+ JVM | 1GB+ JVM |
| Language | SQL | Java/SQL | Java |
| Setup time | 5 minutes | Hours/days | Hours |
| Scale ceiling | ~500k eps | Millions+ | Millions+ |
| Connectors | Growing | 100+ | Kafka ecosystem |

**Choose EventFlux when**: Simple deployment, small-medium scale, SQL preference

**Choose Flink when**: Massive scale, batch+stream, existing JVM infra

**Choose Kafka Streams when**: Already using Kafka, Java team

## Project Structure

```
eventflux-engine/
├── src/
│   ├── core/           # Runtime engine
│   ├── query_api/      # AST and query structures
│   └── sql_compiler/   # SQL parser
├── tests/              # Integration tests (1,400+)
├── examples/           # Example SQL files
└── feat/               # Feature documentation
```

## Contributing

See [DEV_GUIDE.md](DEV_GUIDE.md) for development setup.

1. Fork the repository
2. Create a feature branch
3. Run tests: `cargo test`
4. Submit pull request

## Community

- Issues: [GitHub Issues](https://github.com/eventflux-io/engine/issues)
- Discussions: [GitHub Discussions](https://github.com/eventflux-io/engine/discussions)

## License

Licensed under either of:
- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

## Acknowledgments

EventFlux is inspired by [Apache Siddhi](https://siddhi.io/), reimagined in Rust for simplicity and performance.
