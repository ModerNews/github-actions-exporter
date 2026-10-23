# github-actions-exporter

[![github actions](https://img.shields.io/github/actions/workflow/status/ModerNews/github-actions-exporter/rust.yml?branch=main&style=flat-square&logo=github)](https://github.com/ModerNews/github-actions-exporter/actions)
[![codecov](https://img.shields.io/codecov/c/github/ModerNews/github-actions-exporter?style=flat-square&logo=codecov)](https://codecov.io/gh/ModerNews/github-actions-exporter)

Prometheus exporter for GitHub Actions, driven by organization-level webhooks rather than REST polling.

It consumes `workflow_run` and `workflow_job` deliveries and parses them into prometheus metrics.

**Table of Contents:**

- [Usage](#usage)
  - [Configure the webhook](#configure-the-webhook)
  - [Run the exporter](#run-the-exporter)
  - [Scrape it](#scrape-it)
- [Metrics](#metrics)
- [Installation](#installation)
- [Known limitations](#known-limitations)
- [Development](#development)

## Usage

### Configure the webhook

Add one webhook at the organization level; new repositories then need no per-repo setup:

| Setting | Value |
| --- | --- |
| Payload URL | `https://<host>/api/v1/github` |
| Content type | `application/json` |
| Secret | Same value as `GITHUB_WEBHOOK_SECRET` |
| Events | **Workflow runs** and **Workflow jobs** |

Other events are ignored on arrival.

### Run the exporter

Configuration is read from the environment once at startup. An invalid value fails the process.

| Variable | Default | |
| --- | --- | --- |
| `GITHUB_WEBHOOK_SECRET` | — | **Required.** The hook's shared secret. |
| `BIND_ADDR` | `0.0.0.0:8080` | `host:port` to listen on. |
| `ENTITY_TTL_HOURS` | `24` | Max entity lifetime before eviction. Also bounds how long duplicate deliveries are deduplicated. |
| `RUST_LOG` | `info,tower_http=debug` | `tracing` filter directives. |

```console
$ GITHUB_WEBHOOK_SECRET=... github-actions-exporter
```

| Method | Path | |
| --- | --- | --- |
| `GET` | `/healthz` | Liveness probe. |
| `POST` | `/api/v1/github` | Webhook sink. |
| `GET` | `/api/v1/metrics` | Prometheus text format. |

### Scrape it

```yaml
scrape_configs:
  - job_name: github-actions
    metrics_path: /api/v1/metrics
    static_configs:
      - targets: ['github-actions-exporter:8080']
```

## Metrics

Fourteen metrics in three tiers.

**Terminal**, emitted once per execution when it finishes — `github_actions_jobs_total`,
`github_actions_runs_total`, `github_actions_job_queue_seconds`,
`github_actions_job_duration_seconds`, `github_actions_run_duration_seconds`,
`github_actions_referenced_workflow_runs_total`.

**State**, computed at scrape from observed state — `github_actions_jobs_in_flight`,
`github_actions_runs_in_flight`, `github_actions_queued_job_age_seconds_max`,
`github_actions_workflow_last_completion_timestamp_seconds`.

**Exporter**, about the pipeline itself — `github_actions_exporter_deliveries_total`,
`github_actions_exporter_delivery_lag_seconds`, `github_actions_exporter_state_evictions_total`,
`github_actions_exporter_last_delivery_timestamp_seconds`.

## Installation

Container images are published to GHCR. The version in `Cargo.toml` is the release version: when a push
to `main` carries a version not yet tagged, CI tags it `v<version>` and publishes that image tag
alongside `latest` and `sha-<commit>`. Pushes that do not bump the version publish only the latter two.

```console
$ docker run -p 8080:8080 -e GITHUB_WEBHOOK_SECRET=... \
    ghcr.io/modernews/github-actions-exporter:latest
```

The image is distroless and runs as uid `65532`.

On Kubernetes, with the chart in [`helm/`](helm):

```console
$ helm install github-actions-exporter ./helm \
    --set webhookSecret.value=... \
    --set ingress.enabled=true \
    --set 'ingress.hosts[0].host=<host>'
```

The chart puts only `/api/v1/github` behind the ingress and keeps `/api/v1/metrics` cluster-internal, and
it renders a `ServiceMonitor` on request. Every value is commented in
[`helm/values.yaml`](helm/values.yaml).

With Nix:

```console
$ nix run github:ModerNews/github-actions-exporter
```

From source:

```console
$ cargo build --release
```

## Known limitations

- There is no recovery path for deliveries that failed, or for events never delivered. GitHub does not
  auto-retry and discards failed deliveries after three days, so gaps are permanent.
- An unparseable payload is acknowledged `204`, which marks the delivery `OK` and makes it
  unrecoverable. It is counted as `outcome="parse_error"`.
- In-flight gauges describe observed state, not authoritative GitHub state. A restart empties them until
  traffic refills, and they go stale after missed deliveries.
- Duplicate protection is bounded by `ENTITY_TTL_HOURS`. A redelivery arriving after the entity has been
  evicted is indistinguishable from a first sighting and is counted again.
- Run-level queue delay is not exported. GitHub reports `run_started_at == created_at` on every run
  observed; job-level queue wait is measured instead.

## Development

```console
$ nix develop
$ cargo test
$ cargo clippy --all-targets -- -D warnings
$ cargo fmt --check
$ cargo llvm-cov --summary-only
```

CI runs the suite through [`cargo-nextest`][nextest] to get JUnit output for Codecov's test analytics.
To reproduce that run exactly:

```console
$ NEXTEST_PROFILE=ci cargo llvm-cov nextest --all-features --workspace --codecov --output-path codecov.json
```

[nextest]: https://nexte.st
