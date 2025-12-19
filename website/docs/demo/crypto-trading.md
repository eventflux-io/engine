---
sidebar_position: 1
title: Real-Time Crypto Trading Demo
description: Experience EventFlux CEP with live Bitcoin trades in under a minute
---

# Real-Time Crypto Trading Demo

Experience EventFlux's Complex Event Processing with **real Bitcoin trades** - no setup, no API keys, just two files and one command.

## Quick Start

### 1. Create a new directory

```bash
mkdir eventflux-demo
cd eventflux-demo
```

### 2. Create the query file

Create `query.eventflux` with this content:

```sql title="query.eventflux"
-- EventFlux Advanced Demo: Real-Time Bitcoin Market Analysis
--
-- Architecture:
--   RawTrades (source) ─┬─> ShortTermStats (internal) ─> MarketPulse (sink)
--                       └─> TrendSummary (sink)
--
-- Features demonstrated:
--   - WebSocket source with JSON mapping
--   - Multiple window sizes (5-sec and 30-sec)
--   - Internal stream for pipeline processing
--   - Multiple sink outputs
--   - Multi-query processing

-- =============================================================================
-- STREAMS
-- =============================================================================

-- Source: Real BTC/USDT trades from Binance (no API key required)
CREATE STREAM RawTrades (
    price VARCHAR,
    quantity VARCHAR,
    symbol VARCHAR,
    trade_time BIGINT
) WITH (
    type = 'source',
    extension = 'websocket',
    format = 'json',
    "websocket.url" = 'wss://stream.binance.com:9443/ws/btcusdt@trade',
    "websocket.reconnect" = 'true',
    "websocket.reconnect.max.attempts" = '-1',
    "json.mapping.price" = '$.p',
    "json.mapping.quantity" = '$.q',
    "json.mapping.symbol" = '$.s',
    "json.mapping.trade_time" = '$.T'
);

-- Internal: Short-term statistics (5-second windows)
-- This intermediate stream enables pipeline processing
CREATE STREAM ShortTermStats (
    symbol VARCHAR,
    trade_count BIGINT
);

-- Sink 1: Market pulse - real-time activity from short-term analysis
CREATE STREAM MarketPulse (
    symbol VARCHAR,
    trades_per_5sec BIGINT,
    activity VARCHAR
) WITH (
    type = 'sink',
    extension = 'log',
    format = 'json'
);

-- Sink 2: Trend summary - longer-term 30-second view
CREATE STREAM TrendSummary (
    symbol VARCHAR,
    trades_per_30sec BIGINT
) WITH (
    type = 'sink',
    extension = 'log',
    format = 'json'
);

-- =============================================================================
-- QUERIES
-- =============================================================================

-- Query 1: Aggregate raw trades into 5-second windows
-- Feeds the internal ShortTermStats stream
INSERT INTO ShortTermStats
SELECT
    symbol,
    COUNT(*) AS trade_count
FROM RawTrades
WINDOW('tumbling', INTERVAL '5' SECOND)
GROUP BY symbol;

-- Query 2: Process short-term stats and output to MarketPulse
-- Classifies activity level based on trade frequency
INSERT INTO MarketPulse
SELECT
    symbol,
    trade_count AS trades_per_5sec,
    'ACTIVE' AS activity
FROM ShortTermStats;

-- Query 3: Longer-term trend analysis (30-second windows)
-- Provides a smoother view of market activity
INSERT INTO TrendSummary
SELECT
    symbol,
    COUNT(*) AS trades_per_30sec
FROM RawTrades
WINDOW('tumbling', INTERVAL '30' SECOND)
GROUP BY symbol;
```

### 3. Create the Docker Compose file

Create `docker-compose.yml` with this content:

```yaml title="docker-compose.yml"
services:
  eventflux:
    image: ghcr.io/eventflux-io/engine:latest
    container_name: eventflux-crypto-demo
    command: ["/app/query.eventflux"]
    volumes:
      - ./query.eventflux:/app/query.eventflux:ro
    environment:
      - RUST_LOG=info
    restart: unless-stopped
```

### 4. Run the demo

```bash
docker compose up
```

That's it! Within seconds you'll see real trade data flowing through multiple processing pipelines.

## What You'll See

The demo outputs two types of summaries:

**Market Pulse (every 5 seconds):**
```
[LOG] {"_timestamp":1703001234567,"symbol":"BTCUSDT","trades_per_5sec":340,"activity":"ACTIVE"}
[LOG] {"_timestamp":1703001239567,"symbol":"BTCUSDT","trades_per_5sec":456,"activity":"ACTIVE"}
```

**Trend Summary (every 30 seconds):**
```
[LOG] {"_timestamp":1703001260000,"symbol":"BTCUSDT","trades_per_30sec":1850}
```

| Field | Description |
|-------|-------------|
| **_timestamp** | Event processing time (Unix ms) |
| **symbol** | Trading pair (BTCUSDT) |
| **trades_per_5sec** | Trades in the 5-second window |
| **trades_per_30sec** | Trades in the 30-second window |
| **activity** | Activity classification |

## Architecture

```
┌─────────────────┐     ┌─────────────────────────────────────────────────┐     ┌─────────────────┐
│     Binance     │────▶│                  EventFlux                      │────▶│     Console     │
│    WebSocket    │     │                                                 │     │                 │
│                 │     │  RawTrades ─┬─> 5-sec window ─> ShortTermStats  │     │  Market Pulse   │
│  Real BTC/USDT  │     │             │                   ↓               │     │  (every 5 sec)  │
│     trades      │     │             │              MarketPulse ─────────────▶│                 │
│                 │     │             │                                   │     │  Trend Summary  │
│                 │     │             └─> 30-sec window ─> TrendSummary ──────▶│  (every 30 sec) │
└─────────────────┘     └─────────────────────────────────────────────────┘     └─────────────────┘
```

### Stream Types

| Stream | Type | Purpose |
|--------|------|---------|
| **RawTrades** | Source | Receives real-time trades from Binance WebSocket |
| **ShortTermStats** | Internal | Intermediate processing - enables pipeline architecture |
| **MarketPulse** | Sink | Outputs 5-second activity analysis |
| **TrendSummary** | Sink | Outputs 30-second trend data |

### Query Pipeline

1. **Query 1**: Aggregates raw trades into 5-second windows → `ShortTermStats`
2. **Query 2**: Processes internal stream, adds classification → `MarketPulse`
3. **Query 3**: Parallel 30-second aggregation → `TrendSummary`

## Directory Structure

Your demo directory should look like this:

```
eventflux-demo/
├── docker-compose.yml
└── query.eventflux
```

## Try Different Trading Pairs

Edit `query.eventflux` and change the WebSocket URL:

| Pair | URL |
|------|-----|
| BTC/USDT | `wss://stream.binance.com:9443/ws/btcusdt@trade` |
| ETH/USDT | `wss://stream.binance.com:9443/ws/ethusdt@trade` |
| SOL/USDT | `wss://stream.binance.com:9443/ws/solusdt@trade` |
| XRP/USDT | `wss://stream.binance.com:9443/ws/xrpusdt@trade` |

After editing, restart with:

```bash
docker compose down
docker compose up
```

## Troubleshooting

### No output after 5 seconds

- **Internet connectivity**: Ensure you can reach `stream.binance.com`
- **Firewall**: Some corporate networks block WebSocket connections
- **Region**: Binance may be restricted in some countries

### Container fails to start

```bash
# Pull the latest image
docker pull ghcr.io/eventflux-io/engine:latest

# Check for errors
docker compose logs
```

### Permission denied on query file

```bash
chmod 644 query.eventflux
```

## What's Next?

Now that you've seen EventFlux in action:

- **[Quick Start Guide](/docs/getting-started/quick-start)** - Learn the fundamentals
- **[SQL Reference](/docs/sql-reference/queries)** - Master the query language
- **[WebSocket Connector](/docs/connectors/websocket)** - Build custom integrations
- **[RabbitMQ Connector](/docs/connectors/rabbitmq)** - Connect to message queues

## Why Binance WebSocket?

- **No authentication** - Connect and receive data immediately
- **High volume** - Hundreds of trades per second
- **Free** - No API keys or subscriptions
- **Reliable** - Well-documented, stable API
