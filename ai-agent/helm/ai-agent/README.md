# ai-agent Helm Chart

This chart deploys the `ai-agent` Rust binary as a Kubernetes `Deployment`.

## Quick start

```bash
# From ai-agent/
docker build -t your-registry/ai-agent:0.1.0 .
docker push your-registry/ai-agent:0.1.0

helm upgrade --install ai-agent ./helm/ai-agent \
  --set image.repository=your-registry/ai-agent \
  --set image.tag=0.1.0 \
  --set env.APP_MODE=api \
  --set env.PORT=8080 \
  --set env.LLM_PROVIDER=anthropic \
  --set env.ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY"
```

API endpoints:

- `GET /healthz`
- `POST /v1/swarm/run` with JSON body: `{"input":"..."}`.
- `POST /v1/jobs` with JSON body: `{"input":"..."}`.
- `GET /v1/jobs/:id` for async job status/result.

## Notes

- If you already manage secrets externally, set:
  - `existingSecret.enabled=true`
  - `existingSecret.name=<your-secret-name>` containing `ANTHROPIC_API_KEY` and/or `OPENAI_API_KEY`
- To use OpenAI:
  - `env.LLM_PROVIDER=openai`
  - `env.OPENAI_API_KEY=<your-openai-key>` (or provide it in `existingSecret`)
- For role-based routing, set extra env vars:
  - `env.extra.WORKER_RESEARCHER_LLM_PROVIDER=openai`
  - `env.extra.WORKER_RESEARCHER_MODEL=gpt-4o-mini`
  - `env.extra.SUPERVISOR_LLM_PROVIDER=anthropic`
  - `env.extra.SUPERVISOR_MODEL=claude-sonnet-4-20250514`
- Use `env.APP_MODE=api` for HTTP service mode, or `env.APP_MODE=cli` for one-shot CLI mode.
- `service.enabled` and `ingress.enabled` are optional and disabled by default.
