# AgentSwarm

This repository contains a Rust agent (`ai-agent`) with Docker and Helm packaging so you can run it in Kubernetes environments like AKS, Azure-hosted clusters, or local k3d.

## Swarm architecture

The agent runtime now supports a swarm pattern:

- Multiple worker agents run with different roles.
- Each worker writes its result to a shared blackboard.
- A supervisor agent reads blackboard entries and produces one final decision.

Key modules:

- `ai-agent/src/swarm.rs` - worker orchestration + supervisor handoff.
- `ai-agent/src/blackboard.rs` - shared structured contribution store.
- `ai-agent/src/supervisor.rs` - final decision synthesis logic.

Run locally:

```bash
cd ai-agent
cargo run -- "Evaluate whether Rust is a good fit for a CLI data pipeline."
```

Run as API service:

```bash
cd ai-agent
APP_MODE=api PORT=8080 cargo run
```

Then call:

```bash
curl -X POST http://localhost:8080/v1/swarm/run \
  -H "content-type: application/json" \
  -d '{"input":"Analyze this listing investment risk."}'
```

Or use async jobs:

```bash
# Create a job
curl -X POST http://localhost:8080/v1/jobs \
  -H "content-type: application/json" \
  -d '{"input":"Analyze this listing investment risk."}'
```

```bash
# Poll status/result
curl http://localhost:8080/v1/jobs/job-1
```

## LLM provider plug-and-play

The runtime supports multiple providers via environment variables:

- `LLM_PROVIDER=anthropic` (default) or `LLM_PROVIDER=openai`
- Anthropic:
  - `ANTHROPIC_API_KEY`
  - Optional: `ANTHROPIC_MODEL` (default `claude-sonnet-4-20250514`)
- OpenAI:
  - `OPENAI_API_KEY`
  - Optional: `OPENAI_MODEL` (default `gpt-4o-mini`)
- Role-specific overrides:
  - Worker role provider: `WORKER_<ROLE>_LLM_PROVIDER`
  - Worker role model: `WORKER_<ROLE>_MODEL`
  - Supervisor provider: `SUPERVISOR_LLM_PROVIDER`
  - Supervisor model: `SUPERVISOR_MODEL`
  - Role names are uppercased and non-alphanumeric characters become `_`

Examples:

```bash
# Anthropic
cd ai-agent
LLM_PROVIDER=anthropic ANTHROPIC_API_KEY=... cargo run -- "Analyze this architecture."
```

```bash
# OpenAI
cd ai-agent
LLM_PROVIDER=openai OPENAI_API_KEY=... cargo run -- "Analyze this architecture."
```

```bash
# Mixed swarm: OpenAI researcher, Anthropic supervisor
cd ai-agent
LLM_PROVIDER=anthropic \
ANTHROPIC_API_KEY=... \
OPENAI_API_KEY=... \
WORKER_RESEARCHER_LLM_PROVIDER=openai \
WORKER_RESEARCHER_MODEL=gpt-4o-mini \
SUPERVISOR_LLM_PROVIDER=anthropic \
SUPERVISOR_MODEL=claude-sonnet-4-20250514 \
cargo run -- "Analyze this architecture."
```

## Project layout

- `ai-agent/` - Rust application source.
- `ai-agent/Dockerfile` - Multi-stage container build for the agent binary.
- `ai-agent/helm/ai-agent/` - Helm chart for Kubernetes deployment.

## Prerequisites

- Docker
- Kubernetes cluster (AKS, Azure Kubernetes Service, k3d, or similar)
- `kubectl`
- `helm`
- A container registry (Docker Hub, GHCR, ACR, etc.)
- LLM API key for your selected provider (`ANTHROPIC_API_KEY` or `OPENAI_API_KEY`)

## Build and push the container image

Run from `ai-agent/`:

```bash
docker build -t <registry>/ai-agent:0.1.0 .
docker push <registry>/ai-agent:0.1.0
```

Example with Azure Container Registry (ACR):

```bash
docker build -t myregistry.azurecr.io/ai-agent:0.1.0 .
docker push myregistry.azurecr.io/ai-agent:0.1.0
```

## Deploy with Helm

From the repository root:

```bash
helm upgrade --install ai-agent ./ai-agent/helm/ai-agent \
  --namespace ai-agent --create-namespace \
  --set image.repository=<registry>/ai-agent \
  --set image.tag=0.1.0 \
  --set env.APP_MODE=api \
  --set env.PORT=8080 \
  --set env.LLM_PROVIDER=anthropic \
  --set env.ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  --set args[0]="Summarize this repository."
```

## Deploy using an existing Kubernetes Secret

If you do not want to pass the API key on the command line:

```bash
kubectl create secret generic ai-agent-secret \
  --from-literal=ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -n ai-agent
```

Then install:

```bash
helm upgrade --install ai-agent ./ai-agent/helm/ai-agent \
  --namespace ai-agent --create-namespace \
  --set image.repository=<registry>/ai-agent \
  --set image.tag=0.1.0 \
  --set existingSecret.enabled=true \
  --set existingSecret.name=ai-agent-secret \
  --set args[0]="Summarize this repository."
```

## Verify deployment

```bash
kubectl get pods -n ai-agent
kubectl logs -n ai-agent deploy/ai-agent -f
```

## Notes for AKS / Azure / k3d

- AKS with private registry: configure image pull credentials or attach ACR to AKS.
- k3d local testing: push image to a registry your cluster can access, or import the local image into the k3d cluster.
- The current `ai-agent` binary is CLI-oriented (it processes input and exits). The Helm chart therefore runs it as a workload with `args`, not as a long-running HTTP API.
- `Service` and `Ingress` templates exist in the chart but are disabled by default because the app does not expose an HTTP port yet.
