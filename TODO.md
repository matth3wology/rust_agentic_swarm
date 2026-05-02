# What’s Missing for an Enterprise Agent Swarm
Top gaps, in priority order:

### Service Interface / Runtime Model
It’s still CLI-driven, not a long-running API service (no REST/gRPC job API, no async job status, no webhooks).

### Durable State
Blackboard and memory are in-process only; no persistent DB/event store for audit/history/replay.

### Workflow Reliability
No retries/backoff policies per tool/LLM step, no circuit breakers, no dead-letter handling, no idempotency keys.

### Security & Governance
No tenant isolation, RBAC, policy enforcement, tool allowlists by role, data classification/redaction, or approval gates.

### Observability
You have basic tracing init, but no structured distributed tracing strategy, metrics, dashboards, SLOs, or cost telemetry.

### Human-in-the-loop Controls
No pause/review/approve flows for high-risk actions or low-confidence supervisor outcomes.

### Model/Provider Abstraction Maturity
Single provider + static model; no dynamic routing, fallback providers, or model policy by task sensitivity/cost.

### Evaluation & Quality System
Unit tests exist, but no eval harness, benchmark datasets, regression scoring, or automated quality gates for agent behavior.

### Concurrency and Scaling Controls
Parallel workers exist, but no global concurrency limits, queue-based scheduling, admission control, or per-tenant quotas.

### Production Kubernetes Hardening
Helm is a good start, but no readiness for enterprise ops: PDB, network policies, external secret manager integration patterns, or HPA driven by meaningful custom metrics.

### Tool Safety
file_io is powerful and unsandboxed; enterprise needs strict path sandboxing, capability scoping, and execution isolation.
