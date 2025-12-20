# Zero-Friction Demo

Feature tracking for the self-contained demo that showcases EventFlux pattern processing with real Bitcoin trades.

## Implementation

**Demo files**: `examples/demo/`
- `docker-compose.yml` - Docker Compose configuration
- `query.eventflux` - EventFlux query with pattern processing for trend detection

**Documentation**: `website/docs/demo/crypto-trading.md`
- Step-by-step copy-paste instructions for users
- Pattern processing explanation with semantic diagrams

## Status

- [x] WebSocket source connector
- [x] Self-contained demo files
- [x] Website documentation with copy-paste guide
- [x] Sidebar entry in "Demos" category
- [x] env_logger initialization for log output
- [x] LogSink JSON format support
- [x] Explicit JSON field mappings for Binance data
- [x] JSON sink output with proper schema field names
- [x] Test on fresh Docker environment (verified working)
- [x] **PATTERN PROCESSING** - Same-stream pattern matching
- [x] EVERY modifier for continuous pattern matching
- [x] Arithmetic expressions in pattern queries (e2.count - e1.count)
- [x] Trend detection (activity_change = curr_trades - prev_trades)
- [x] Multi-query pipeline (source -> window -> internal -> pattern -> sink)
- [x] **CAST expression** - Type conversion (VARCHAR to DOUBLE)
- [x] Average price calculation using CAST
- [x] Price change percentage in trend signal
- [ ] Add "Try it" section to main README.md

## Technical Notes

### Pattern Processing
- Uses same-stream pattern: `EVERY(e1=ActivityStats -> e2=ActivityStats)`
- Pattern aliases (e1, e2) reference consecutive events from the same stream
- EVERY modifier ensures continuous matching after each completion
- Arithmetic expressions work with pattern aliases: `e2.trade_count - e1.trade_count`
- Price change percentage: `((e2.avg_price - e1.avg_price) / e1.avg_price) * 100.0`

### CAST Expression
- Binance sends price as VARCHAR (string)
- Use `CAST(price AS DOUBLE)` to convert for numeric operations
- Enables `AVG(CAST(price AS DOUBLE))` for average price calculation

### Data Flow
- Binance sends price/quantity as strings, not numbers
- JSON mapper sorts fields alphabetically - schema must match this order
- Uses `json.mapping.*` properties for explicit JSONPath extraction
- Internal streams enable pipeline processing (source -> internal -> sink)

### Pattern Output Timing
- First ActivityPulse appears after ~10 seconds (first window completes)
- First TrendSignal appears after ~20 seconds (pattern needs 2 consecutive events)

## Architecture

```
RawTrades (source)
    │
    ├─[10-sec tumbling window]
    │  COUNT(*), AVG(CAST(price AS DOUBLE))
    │
    ▼
ActivityStats (internal)
    │  - trade_count
    │  - avg_price
    │
    ├───────────────────────────┐
    │                           │
    ▼                           ▼
ActivityPulse (sink)    PATTERN MATCHING
- trades                EVERY(e1 -> e2)
- avg_price                    │
                               ▼
                        TrendSignal (sink)
                        - prev_trades / curr_trades
                        - activity_change
                        - prev_price / curr_price
                        - price_change_pct
```
