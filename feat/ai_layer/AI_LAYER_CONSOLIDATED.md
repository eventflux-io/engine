# AI Layer - Consolidated Design and Implementation

Last Updated: 2025-11-20
Status: Consolidated from multiple design documents

---

## 1. Purpose and Scope

EventFlux AI Layer provides integration between high-frequency event streams and LLM-based decision making. It enables pattern detection at millions of events per second, with intelligent routing between rule-based decisions and AI escalation for complex cases.

The layer addresses the performance and cost mismatch between event streams (sub-millisecond latency, millions of events/second) and LLM calls (1-5 second latency, $0.001-$0.01 per call).

---

## 2. Design Philosophy

Final decision: Configure in TOML, Control in SQL.

AI agents follow the same pattern as existing EventFlux extensions:
- Sources/Sinks: Configured in TOML, referenced in SQL
- Tables: Configured in TOML/SQL WITH, queried in SQL
- AI Agents: Configured in TOML, called as functions in SQL
- Action Handlers: Configured in TOML, triggered via SQL

This approach was selected over inline SQL definitions because:
- No SQL parser changes required for MVP
- Clean separation between configuration and business logic
- AI engineers can iterate on prompts without modifying SQL
- Follows existing EventFlux conventions
- Production-ready architecture from initial release

---

## 3. Architecture

### 3.1 Three-Layer Model

```
Layer 3: AI/LLM Layer
- Complex decisions requiring reasoning
- Novel situations
- High-value actions
- Volume: 10-100 decisions/second (0.01% of events)
- Latency: 1-5 seconds

Layer 2: EventFlux Intelligence Layer
- Pattern detection (real-time CEP)
- Context aggregation
- Hybrid decision engine (rules + AI routing)
- Action execution
- Volume: 1M events/sec -> 100 patterns/sec
- Latency: <10ms

Layer 1: Event Sources
- IoT sensors, transactions, user actions, logs
- Volume: 1M-100M events/second
- Intelligence: None
```

### 3.2 Pipeline Enhancement

Existing pipeline remains unchanged. New processors added:

```
StreamJunction (unchanged)
    |
Processor Chain:
    |- FilterProcessor (unchanged)
    |- SelectProcessor (unchanged)
    |- JoinProcessor (unchanged)
    |- WindowProcessor (unchanged)
    |- PatternProcessor (unchanged)
    |- RuleEvaluationProcessor (NEW)
    |- AIDecisionProcessor (NEW)
    |- DecisionRouterProcessor (NEW)
    |- ActionExecutorProcessor (NEW)
```

### 3.3 Async Request/Response Model

LLMs are async services, not synchronous transformations. The architecture treats AI agents as request/response services with:
- Request Sink: Send events to AI agent
- Response Source: Receive responses from AI agent
- Correlation ID: Match responses to original requests

Flow:
```
Event -> AI Request Sink (non-blocking) -> Continue processing
                |
                v (1-5 seconds later)
         AI Service processes
                |
                v
AI Response Source -> Correlation match -> Action execution
```

---

## 4. Component Specifications

### 4.1 LLM Provider

Location: src/core/ai/llm_provider.rs

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn send_prompt(&self, prompt: String, config: &AIAgentConfig)
        -> Result<LLMResponse, LLMError>;
    fn get_cost_per_token(&self) -> f64;
    fn get_max_tokens(&self) -> usize;
    fn get_provider_name(&self) -> &str;
}
```

Required implementations:
- AnthropicProvider
- OpenAIProvider
- OllamaProvider (local models)

### 4.2 AI Agent

Location: src/core/ai/agent.rs

```rust
pub struct AIAgent {
    name: String,
    provider: Box<dyn LLMProvider>,
    config: AIAgentConfig,
    prompt_template: PromptTemplate,
    response_parser: Box<dyn ResponseParser>,
    context_queries: HashMap<String, String>,
    response_schema: Vec<(String, String)>,
    cost_tracker: Arc<Mutex<CostTracker>>,
    cache: Option<Arc<LRUCache<String, AIResponse>>>,
}

pub struct AIAgentConfig {
    model: String,
    max_tokens: usize,
    temperature: f64,
    timeout_ms: u64,
    max_cost_per_hour: f64,
    max_cost_per_day: f64,
    enable_caching: bool,
    cache_ttl_minutes: u64,
    cache_max_size_mb: usize,
    response_format: ResponseFormat,
}
```

Default configuration values:
- model: "claude-3-5-sonnet-20241022"
- max_tokens: 1000
- temperature: 0.3
- timeout_ms: 5000
- max_cost_per_hour: 5.0
- max_cost_per_day: 50.0
- enable_caching: true
- cache_ttl_minutes: 60
- cache_max_size_mb: 100

### 4.3 AI Agent Registry

Location: src/core/ai/agent_registry.rs

```rust
pub struct AIAgentRegistry {
    agents: RwLock<HashMap<String, Arc<AIAgent>>>,
}

impl AIAgentRegistry {
    pub fn new() -> Self;
    pub fn register(&self, name: String, agent: AIAgent);
    pub fn get(&self, name: &str) -> Option<Arc<AIAgent>>;
    pub fn load_from_config(&self, config_path: &str) -> Result<(), String>;
}
```

### 4.4 Context Aggregator

Location: src/core/ai/context_aggregator.rs

```rust
pub struct ContextAggregator {
    context_queries: HashMap<String, String>,
    eventflux_manager: Arc<EventFluxManager>,
}

impl ContextAggregator {
    pub async fn build_context(&self, base_event: &StreamEvent, context_spec: &ContextSpec)
        -> Result<EventContext, ContextError>;
}
```

Context queries are SQL statements with {{variable}} substitution executed before AI calls.

### 4.5 Cost Tracker

Location: src/core/ai/cost_tracker.rs

```rust
pub struct CostTracker {
    hourly_cost: f64,
    daily_cost: f64,
    hourly_tokens: usize,
    daily_tokens: usize,
    hourly_calls: usize,
    daily_calls: usize,
    last_hourly_reset: Instant,
    last_daily_reset: Instant,
}

impl CostTracker {
    pub fn new() -> Self;
    pub fn add_call(&mut self, tokens: usize, cost: f64);
    pub fn is_within_budget(&mut self, config: &AIAgentConfig) -> bool;
}
```

Budget enforcement resets hourly (3600 seconds) and daily (86400 seconds).

### 4.6 Prompt Template

Location: src/core/ai/prompt_template.rs

Uses Handlebars template syntax.

```rust
pub struct PromptTemplate {
    template: String,
    handlebars: Handlebars<'static>,
}

impl PromptTemplate {
    pub fn from_file(path: &str) -> Result<Self, String>;
    pub fn new(template: String) -> Result<Self, String>;
    pub fn render(&self, data: &serde_json::Value) -> Result<String, String>;
}
```

### 4.7 AI Decision Executor

Location: src/core/executor/function/ai_decision_executor.rs

```rust
pub struct AIDecisionExecutor {
    agent_name: String,
    param_executors: Vec<Box<dyn ExpressionExecutor>>,
    agent_registry: Arc<AIAgentRegistry>,
    fallback: StructValue,
}

impl ExpressionExecutor for AIDecisionExecutor {
    fn execute(&self, event: Option<&StreamEvent>) -> Option<AttributeValue>;
    fn get_return_type(&self) -> Type;
}
```

Execution sequence:
1. Get agent from registry
2. Extract parameters from event
3. Check budget
4. Check cache
5. Build context
6. Render prompt
7. Call LLM
8. Parse response
9. Cache result
10. Track cost
11. Return AttributeValue::Struct

On any failure (budget exceeded, timeout, parse error), returns fallback struct.

### 4.8 Rule Engine

Location: src/core/decision/rule_engine.rs

```rust
pub struct Rule {
    name: String,
    condition: Arc<dyn ExpressionExecutor>,
    action_expr: Arc<dyn ExpressionExecutor>,
    confidence_expr: Arc<dyn ExpressionExecutor>,
    reasoning_expr: Arc<dyn ExpressionExecutor>,
}

pub struct RuleEngine {
    rules: Vec<Rule>,
    evaluation_strategy: EvaluationStrategy,
}

pub enum EvaluationStrategy {
    FirstMatch,
    HighestConfidence,
    Combine,
}
```

Rules compile to EventFlux ExpressionExecutor for native speed execution.

### 4.9 Decision Stream Processor

Location: src/core/query/processor/decision_stream_processor.rs

```rust
pub struct DecisionStreamProcessor {
    meta: CommonProcessorMeta,
    rule_engine: Arc<RuleEngine>,
    ai_agent: Option<Arc<AIAgent>>,
    context_aggregator: Arc<ContextAggregator>,
    escalation_strategy: EscalationStrategy,
    escalation_threshold: f64,
    fallback_action: String,
    fallback_confidence: f64,
}

pub enum EscalationStrategy {
    ConfidenceThreshold(f64),
    Always,
    Never,
    CostBudget(f64),
}
```

### 4.10 Action Handler

Location: src/core/action/action_handler.rs

```rust
#[async_trait]
pub trait ActionHandler: Send + Sync {
    async fn execute(&self, event: &StreamEvent, params: &HashMap<String, String>)
        -> Result<ActionResult, ActionError>;
    fn get_handler_type(&self) -> &str;
}
```

Built-in handlers:
- DatabaseActionHandler
- HttpActionHandler
- KafkaActionHandler

---

## 5. Configuration Format

### 5.1 AI Provider Configuration

File: ai_config.toml

```toml
[ai_providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
base_url = "https://api.anthropic.com"
default_model = "claude-3-5-sonnet-20241022"

[ai_providers.openai]
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
default_model = "gpt-4-turbo-preview"

[ai_providers.local]
backend = "ollama"
base_url = "http://localhost:11434"
default_model = "llama3:70b"
```

### 5.2 Budget Configuration

```toml
[ai_budget]
daily_limit = 100.00
hourly_limit = 10.00
per_call_limit = 0.10
alert_threshold = 0.8
```

### 5.3 AI Agent Configuration

```toml
[ai_agents.fraud_detector]
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
prompt_template_file = "prompts/fraud_analysis.hbs"

# Performance
max_tokens = 1000
temperature = 0.3
timeout_ms = 5000

# Cost controls
max_cost_per_hour = 5.00
max_cost_per_day = 50.00

# Caching
enable_caching = true
cache_ttl_minutes = 60
cache_max_size_mb = 100

# Response format
response_format = "json"

[ai_agents.fraud_detector.response_schema]
action = "string"
confidence = "number"
reasoning = "string"
risk_factors = "array"
recommended_followup = "string"

# Context queries
[ai_agents.fraud_detector.context]
user_profile = """
    SELECT customer_since, avg_transaction_amount,
           typical_locations, typical_merchants, risk_level
    FROM UserProfiles
    WHERE user_id = {{user_id}}
"""

recent_transactions = """
    SELECT amount, merchant, location, timestamp
    FROM Transactions
    WHERE user_id = {{user_id}}
      AND timestamp > NOW() - INTERVAL '24' HOUR
    ORDER BY timestamp DESC
    LIMIT 10
"""
```

### 5.4 Async Mode Configuration

```toml
[ai_agents.fraud_detector]
mode = "async"
request_sink = "FraudDetectorRequests"
response_source = "FraudDetectorResponses"
correlation_ttl_seconds = 300
max_pending_requests = 10000

# Batching
batch_size = 100
batch_timeout_ms = 1000
```

### 5.5 Action Handler Configuration

File: action_handlers.toml

```toml
[action_handlers.block_transaction]
type = "database"
connection = "postgres_fraud_db"
query = """
    UPDATE transactions
    SET status = 'BLOCKED',
        blocked_reason = $1,
        blocked_at = NOW(),
        decision_confidence = $2
    WHERE transaction_id = $3
"""
params = ["{{reasoning}}", "{{confidence}}", "{{transaction_id}}"]
retry_count = 3
retry_backoff = "exponential"
timeout_ms = 1000
audit_log = true

[action_handlers.contact_customer]
type = "http"
url = "https://api.company.com/notifications/fraud-alert"
method = "POST"
timeout_ms = 5000
retry_count = 3

[action_handlers.contact_customer.headers]
Authorization = "Bearer ${NOTIFICATION_API_KEY}"
Content-Type = "application/json"

[action_handlers.contact_customer.body]
user_id = "{{user_id}}"
transaction_id = "{{transaction_id}}"
alert_type = "fraud_suspicion"
severity = "HIGH"
message = "{{reasoning}}"
channels = ["sms", "email", "push"]

[action_handlers.manual_review]
type = "kafka"
brokers = "localhost:9092"
topic = "fraud_review_queue"

[action_handlers.manual_review.message]
transaction_id = "{{transaction_id}}"
user_id = "{{user_id}}"
decision_confidence = "{{confidence}}"
reasoning = "{{reasoning}}"
assigned_to = "fraud_team"
priority = "HIGH"
```

### 5.6 Prompt Template Format

File: prompts/fraud_analysis.hbs (Handlebars syntax)

```handlebars
You are an expert fraud detection specialist.

## User Profile
- User ID: {{user_id}}
- Customer Since: {{context.user_profile.customer_since}}
- Average Transaction: ${{context.user_profile.avg_transaction_amount}}
- Typical Locations: {{join context.user_profile.typical_locations ", "}}

## Current Transaction
- Amount: ${{amount}}
- Merchant: {{merchant}} ({{merchant_category}})
- Location: {{location}}

## Risk Indicators Detected
{{#if has_geo_anomaly}}
- Geographic Anomaly: Traveled {{distance_miles}} miles at {{velocity_mph}} mph
{{/if}}
{{#if has_spending_anomaly}}
- Spending Spike: {{spending_ratio}}x normal spending
{{/if}}

## Recent Transaction History
{{#each context.recent_transactions}}
- ${{this.amount}} at {{this.merchant}} ({{this.location}}) - {{this.timestamp}}
{{/each}}

## Preliminary Risk Score
{{risk_score}} (0-1 scale)

---

Respond in JSON format:
{
  "action": "ALLOW | BLOCK | CONTACT_CUSTOMER | MANUAL_REVIEW",
  "confidence": 0.0-1.0,
  "reasoning": "Explanation",
  "risk_factors": ["specific concerns"],
  "recommended_followup": "next steps"
}
```

---

## 6. SQL Interface

### 6.1 AI Decision Function

ai_decide() is a built-in function like COUNT or SUM.

Syntax:
```sql
ai_decide('agent_name', param1, param2, ...)
```

Returns: STRUCT with fields defined in response_schema

Usage:
```sql
SELECT
    transaction_id,
    ai_decide('fraud_detector',
              transaction_id, user_id, amount, merchant, risk_score) as decision
FROM RiskAssessment
WHERE risk_score > 0.5;
```

### 6.2 Field Access from Result

```sql
SELECT
    transaction_id,
    decision.action,
    decision.confidence,
    decision.reasoning
FROM (
    SELECT
        transaction_id,
        ai_decide('fraud_detector', transaction_id, user_id, amount) as decision
    FROM RiskAssessment
);
```

### 6.3 Hybrid Decision Pattern

Rules provide fast path, AI handles uncertain cases:

```sql
CREATE STREAM FraudDecisions AS
SELECT
    transaction_id,
    user_id,
    CASE
        WHEN velocity_mph > 500 THEN
            struct('BLOCK_TRANSACTION', 1.0, 'Impossible velocity')

        WHEN risk_score > 0.8 AND (has_geo_anomaly + has_spending_anomaly) >= 2 THEN
            struct('MANUAL_REVIEW', 0.85, 'Multiple indicators')

        WHEN risk_score BETWEEN 0.5 AND 0.8 THEN
            ai_decide('fraud_detector', transaction_id, user_id, amount,
                      merchant, risk_score, velocity_mph, spending_ratio)

        ELSE struct('ALLOW', 1.0, 'No risk')
    END as decision
FROM RiskAssessment;
```

### 6.4 Action Execution

Actions triggered via INSERT with action_handler:

```sql
INSERT INTO BlockedTransactions
SELECT transaction_id, user_id, reasoning, confidence
FROM FraudDecisions
WHERE action = 'BLOCK_TRANSACTION'
WITH (
    'action_handler' = 'block_transaction'
);

INSERT INTO CustomerAlerts
SELECT transaction_id, user_id, reasoning
FROM FraudDecisions
WHERE action = 'CONTACT_CUSTOMER'
WITH (
    'action_handler' = 'contact_customer'
);
```

---

## 7. Async Model and Request/Response Correlation

### 7.1 Correlation Management

For each async AI agent, EventFlux creates a pending requests table:

```sql
CREATE TABLE _ai_pending_requests_{agent_name} (
    correlation_id STRING PRIMARY KEY,
    -- Original request fields stored for join
    transaction_id STRING,
    user_id STRING,
    amount DOUBLE,
    -- etc
    sent_at TIMESTAMP,
    ttl TIMESTAMP
);
```

Background task cleans expired requests based on correlation_ttl_seconds config.

### 7.2 Async Sink/Source Pattern

Request sink:
```sql
INSERT INTO FraudDetectorRequests
SELECT
    uuid() as correlation_id,
    transaction_id,
    user_id,
    amount,
    merchant,
    risk_score
FROM ModerateRiskForAI;
```

Response join:
```sql
CREATE STREAM AIDecisions AS
SELECT
    p.transaction_id,
    p.user_id,
    r.action,
    r.confidence,
    r.reasoning
FROM FraudDetectorResponses r
JOIN _ai_pending_requests_fraud_detector p
  ON r.correlation_id = p.correlation_id;
```

Timeout handling:
```sql
CREATE STREAM AITimeouts AS
SELECT
    p.correlation_id,
    p.transaction_id,
    'MANUAL_REVIEW' as action,
    0.5 as confidence,
    'AI timeout' as reasoning
FROM _ai_pending_requests_fraud_detector p
WHERE p.sent_at < NOW() - INTERVAL '30' SECOND
  AND NOT EXISTS (
    SELECT 1 FROM FraudDetectorResponses r
    WHERE r.correlation_id = p.correlation_id
  );
```

### 7.3 Out-of-Order Response Handling

Responses may arrive in different order than requests. Correlation ID matching makes order irrelevant:

```sql
-- Response C arrives first
INSERT INTO FraudDetectorResponses VALUES ('corr-C', 'BLOCK', ...);

-- EventFlux automatically matches
SELECT * FROM _ai_pending_requests_fraud_detector
WHERE correlation_id = 'corr-C';
```

### 7.4 Batching for Cost Optimization

Without batching:
- 1,000 events/sec
- 1,000 API calls/sec
- $86,400/day

With batching (batch_size=100):
- 1,000 events/sec
- 10 API calls/sec (100 events per call)
- $8,640/day (90% cost reduction)

Configuration:
```toml
[ai_agents.fraud_detector]
batch_size = 100
batch_timeout_ms = 1000
```

### 7.5 Partial Batch Failure Handling

```rust
AIBatchResponse {
    successes: vec![
        (correlation_id_1, decision_1),
        // ...
        (correlation_id_98, decision_98),
    ],
    failures: vec![
        (correlation_id_99, "JSON parse error"),
        (correlation_id_100, "Context query timeout"),
    ]
}

// EventFlux handles
for success in successes {
    send_to_response_source(success);
}

for failure in failures {
    send_to_response_source(AIResponse {
        correlation_id: failure.0,
        action: "MANUAL_REVIEW",
        confidence: 0.0,
        reasoning: format!("AI error: {}", failure.1),
    });
}
```

---

## 8. Metrics

### 8.1 AI Metrics

```rust
pub struct AIMetrics {
    // Decision metrics
    pub total_decisions: Counter,
    pub rule_decisions: Counter,
    pub ai_decisions: Counter,
    pub fallback_decisions: Counter,

    // Performance
    pub decision_latency: Histogram,
    pub ai_call_latency: Histogram,
    pub context_build_latency: Histogram,

    // Cost
    pub ai_calls_total: Counter,
    pub ai_cost_total: Gauge,
    pub ai_tokens_used: Counter,
    pub cost_by_agent: HashMap<String, Gauge>,

    // Quality
    pub confidence_distribution: Histogram,
    pub escalation_rate: Gauge,

    // Errors
    pub ai_timeouts: Counter,
    pub ai_errors: Counter,
    pub budget_exceeded_count: Counter,
}
```

### 8.2 Async Metrics

```rust
pub struct AIAgentAsyncMetrics {
    pub requests_sent: Counter,
    pub requests_pending: Gauge,
    pub requests_timeout: Counter,
    pub responses_received: Counter,
    pub responses_latency: Histogram,
    pub responses_out_of_order: Counter,
    pub batch_size_actual: Histogram,
    pub batch_wait_time: Histogram,
    pub correlation_matches: Counter,
    pub correlation_orphaned: Counter,
    pub correlation_expired: Counter,
}
```

### 8.3 Prometheus Export

```
eventflux_ai_decisions_total{agent="fraud_detector",action="BLOCK"} 1234
eventflux_ai_cost_usd{agent="fraud_detector"} 45.67
eventflux_ai_latency_seconds_bucket{le="0.5"} 10
eventflux_ai_latency_seconds_bucket{le="1.0"} 450
eventflux_ai_requests_pending{agent="fraud_detector"} 234
eventflux_ai_correlation_matches_total{agent="fraud_detector"} 9988
```

---

## 9. Implementation Phases

### Phase 1: Core AI Integration (6 weeks)

Week 1-2: LLM Provider abstraction
- trait LLMProvider
- AnthropicProvider implementation
- OpenAIProvider implementation
- OllamaProvider implementation
- Configuration parsing from TOML

Week 3: Prompt template engine
- Handlebars integration
- Template loading and rendering
- Context variable substitution

Week 4: AI Agent registry
- Load agents from ai_config.toml
- Context query execution
- Response parsing (JSON)

Week 5: ai_decide() function
- Function registration in FunctionRegistry
- AIDecisionExecutor (implements ExpressionExecutor)
- Sync execution (blocking await)

Week 6: Cost tracking and budgets
- CostTracker with budget enforcement
- Prometheus metrics export
- Integration tests

Deliverable: SELECT ai_decide('agent', data) FROM Stream works

### Phase 2: Action Handlers (4 weeks)

Week 1: Action handler framework
- trait ActionHandler
- DatabaseActionHandler
- HttpActionHandler
- KafkaActionHandler
- Configuration parsing from TOML

Week 2: Action execution
- ActionExecutorProcessor
- Retry logic with exponential backoff
- Audit logging

Week 3: SQL integration
- WITH ('action_handler' = 'name') syntax
- Template variable substitution
- Error handling

Week 4: Testing and documentation
- Integration tests
- Example applications

Deliverable: Complete fraud detection example works

### Phase 3: Async Sink/Source (6 weeks)

Week 1: Request sink
- AI agent request sink
- Schema inference from agent config
- Send to AI service (async)

Week 2: Response source
- AI agent response source
- Correlation matching
- Join to pending requests

Week 3: Batching
- Batch buffer
- Timeout-based flush
- Cost optimization

Week 4: Correlation management
- Pending requests table
- Background cleanup task
- TTL enforcement

Week 5: Advanced features
- Out-of-order response handling
- Partial batch failure handling
- Memory limits

Week 6: Testing
- Integration tests
- Performance benchmarks

Deliverable: Full async sink/source pattern working

### Phase 4: Production Features (6 weeks)

Week 1-2: Caching
- LRU cache for identical AI inputs
- Cache invalidation strategies
- Cache metrics

Week 3-4: Observability
- Decision quality metrics
- Latency tracking (P50, P95, P99)
- Cost breakdown

Week 5-6: Developer tools
- CLI testing: eventflux ai test agent_name --input data.json
- Prompt validator
- Decision simulator

Deliverable: Production-ready AI layer

---

## 10. File Structure

```
eventflux/
├── src/
│   └── core/
│       ├── ai/
│       │   ├── mod.rs
│       │   ├── llm_provider.rs
│       │   ├── anthropic_provider.rs
│       │   ├── openai_provider.rs
│       │   ├── ollama_provider.rs
│       │   ├── agent_registry.rs
│       │   ├── context_aggregator.rs
│       │   ├── prompt_template.rs
│       │   ├── response_parser.rs
│       │   ├── cost_tracker.rs
│       │   └── metrics.rs
│       ├── decision/
│       │   ├── mod.rs
│       │   ├── rule_engine.rs
│       │   └── decision_router.rs
│       ├── action/
│       │   ├── mod.rs
│       │   ├── action_handler.rs
│       │   ├── database_handler.rs
│       │   ├── http_handler.rs
│       │   ├── kafka_handler.rs
│       │   ├── action_executor_processor.rs
│       │   └── audit_log.rs
│       └── query/
│           └── processor/
│               ├── ai_decision_processor.rs
│               └── decision_stream_processor.rs
└── config/
    ├── ai_config.toml
    ├── action_handlers.toml
    └── prompts/
        └── fraud_analysis.hbs
```

---

## 11. Execution Flow

Transaction arrives:

1. Pattern Detection (RiskAssessment stream)
   - Check geographic velocity
   - Check spending spike
   - Calculate risk_score

2. FraudDecisions stream (Hybrid logic)
   - Evaluate CASE statement
   - velocity_mph > 500? -> BLOCK (rule, confidence 1.0)
   - risk_score > 0.8 + multiple indicators? -> MANUAL_REVIEW (rule, confidence 0.85)
   - risk_score 0.5-0.8? -> ESCALATE TO AI

3. AI escalation (if triggered)
   - Fetch context (user_profile, recent_transactions)
   - Render prompt template
   - Call LLM API (async, 1-2 seconds)
   - Parse JSON response
   - Return decision struct

4. Action execution
   - Route to action handler based on decision.action
   - Execute with retry logic
   - Log to audit trail

---

## 12. Cost Economics

Example: 100K transactions/sec

```
100,000 transactions/sec
    |
5,000/sec trigger risk_score > 0.5 (5%)
    |
Hybrid Decision Routing:
  - 4,500/sec by rules (90%) -> No LLM cost, <1ms
  - 500/sec to AI (10%) -> LLM cost, 1-2 seconds
    |
AI Costs:
  - 500 calls/sec
  - $0.001 per call average
  - $0.50/sec
  - $43,200/day
    |
vs Pure AI:
  - 5,000 calls/sec
  - $5/sec
  - $432,000/day
    |
Savings: 90% cost reduction
```

With 95% rule coverage:
- 250/sec to AI
- $21,600/day
- 95% cost reduction

---

## 13. Error Handling

### AI Errors

- Timeout: Return fallback decision
- Budget exceeded: Return fallback decision
- Parse error: Log and return fallback
- Context query failure: Use defaults or return fallback

### Action Errors

- Retry with exponential backoff
- Log to audit after max retries
- Rollback if configured

### Correlation Errors

- Orphaned response (no matching request): Log and discard
- Expired request (response too late): Log and discard

---

## 14. Testing Requirements

### Unit Tests

- LLMProvider implementations
- PromptTemplate rendering
- ResponseParser extraction
- RuleEngine evaluation
- CostTracker budget enforcement

### Integration Tests

- End-to-end ai_decide() function call
- Context query execution
- Action handler execution
- Async sink/source correlation

### Performance Tests

- Latency under load
- Throughput measurement
- Memory usage with pending requests
- Cache hit rates

### Cost Tests

- Budget enforcement
- Daily/hourly limits
- Per-call limits

---

## 15. Dependencies

Required crates:
- reqwest (HTTP client for LLM APIs)
- handlebars (template rendering)
- serde_json (JSON parsing)
- tokio (async runtime)
- lru (caching)
- prometheus (metrics)

---

## 16. Security Considerations

- API keys stored in environment variables
- Template variables sanitized before SQL execution
- Budget limits prevent runaway costs
- Audit logging for compliance
- No sensitive data in prompts unless necessary

---

## 17. Sync vs Async Usage

### Sync Function (Level 1)

Use for:
- Prototyping / development
- Low volume (<100 AI calls/sec)
- Simple linear flows
- Single consumer of AI results

### Async Sink/Source (Level 2)

Use for:
- Production deployments
- High volume (>1K AI calls/sec)
- Need batching for cost optimization
- Multiple consumers of AI results
- Complex timeout/retry logic
- Monitoring & observability

---

## 18. Context Query Implementation

Context queries from agent config need execution against EventFlux tables.

Initial implementation: Simplified key-value lookup
```rust
fn lookup_table(
    &self,
    table_name: &str,
    key_name: &str,
    key_value: &AttributeValue,
    tables: &TableRegistry,
) -> Result<serde_json::Value, String> {
    let table = tables.get(table_name)?;
    let results = table.find(&[key_value.clone()]);
    // Convert to JSON
}
```

Future expansion: Full SQL query execution with parameter substitution.

---

End of Consolidated Document
