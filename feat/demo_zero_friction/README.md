# Zero-Friction Demo

Feature tracking for the self-contained demo that showcases EventFlux with real Bitcoin trades.

## Implementation

**Demo files**: `examples/demo/`
- `docker-compose.yml` - Docker Compose configuration
- `query.eventflux` - EventFlux query for Binance trades (advanced multi-stream version)

**Documentation**: `website/docs/demo/crypto-trading.md`
- Step-by-step copy-paste instructions for users

## Status

- [x] WebSocket source connector
- [x] Self-contained demo files
- [x] Website documentation with copy-paste guide
- [x] Sidebar entry in "Demos" category
- [x] env_logger initialization for log output
- [x] LogSink JSON format support
- [x] Explicit JSON field mappings for Binance data
- [x] JSON sink output with proper schema field names (symbol, trade_count instead of field_0, field_1)
- [x] Test on fresh Docker environment (verified working)
- [x] Advanced multi-stream demo (source → internal → sink pipeline)
- [x] Multiple window sizes (5-sec and 30-sec)
- [x] Multiple sink outputs (MarketPulse and TrendSummary)
- [x] Multi-query processing (3 INSERT INTO queries)
- [ ] Add "Try it" section to main README.md

## Technical Notes

- Binance sends price/quantity as strings, not numbers
- JSON mapper sorts fields alphabetically - schema must match this order
- Uses `json.mapping.*` properties for explicit JSONPath extraction
- Internal streams enable pipeline processing (source → internal → sink)
- String literals in SELECT work ('ACTIVE' AS activity)
