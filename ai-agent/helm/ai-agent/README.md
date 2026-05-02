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
  --set env.LLM_PROVIDER=anthropic \
  --set env.ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  --set args[0]="Summarize the latest Rust release notes."
```

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
- The current binary is CLI-oriented, so provide request text through `args`.
- `service.enabled` and `ingress.enabled` are optional and disabled by default.
