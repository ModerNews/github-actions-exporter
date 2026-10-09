# Roadmap

What is left to build, in the order the dependencies actually force. Companion to
[`webhook-contract.md`](webhook-contract.md), which is the source of truth for payload shape — most
requirements below are already stated there and are collected here as work items.

Marked **verified** where live traffic from our own org confirmed it, **spec** where only the schema or
GitHub's docs say so. The distinction matters: see the contract doc's note that live payloads carry fields
the schema omits.

## Where we are

Ingest works end to end. Verified 2026-10-09 against `WMS-DEV/kraken` and `WMS-DEV/infra-helm`: both
events parse through their delivery envelope across the full lifecycle (`requested`/`queued` →
`in_progress` → `completed`), and `referenced_workflows` resolves `infra-ci-lib` version pins in
production. Roughly 30 consecutive deliveries, no parse failures.

Everything downstream of parsing is unbuilt. The handler logs the payload and returns `204`; nothing is
counted, exported, stored, or authenticated.

## Phase 1 — required before this is exposed to anything real

### 1.1 HMAC verification of `X-Hub-Signature-256`

The contract doc already states the requirement: *"Verify `X-Hub-Signature-256` (HMAC-SHA256) before
parsing anything."* Today there is no secret, no verification, and the endpoint accepts any POST.

- Compare in **constant time** — a byte-by-byte early return leaks the signature.
- Verify against the **raw body bytes**, before deserialization. The handler already takes `Bytes`, so the
  raw body is available; keep it that way and resist parsing first.
- Reject with `401` (or `403`) and do not parse. GitHub's own validation examples use both. The reason is
  not flow control — it is that an unauthenticated request must not be recorded as successfully
  processed (1.3).
- **Constant time is a documented requirement, not a nicety**: *"Never use a plain `==` operator. Instead
  consider using a method like `secure_compare` … which performs a 'constant time' string comparison to
  help mitigate certain timing attacks."*
- Secret comes from the environment, not a literal. We already run Vault in CI; the deploy story (4.1)
  should pull from the same place.
- Needs a dependency: `hmac` + `sha2`, or `ring`. First new crypto dep in the tree — worth a deliberate
  choice rather than whatever is convenient.

### 1.2 Configuration

The bind address is hardcoded `0.0.0.0:8080`. This blocked testing twice during development: the only way
to run a second instance was to patch the source. Needs env-driven config for at least bind address,
webhook secret, and log filter.

### 1.3 The failure-response contract

Read off GitHub's docs rather than inferred. Three facts, in order of how much they change the design:

1. **GitHub never auto-retries.** *"GitHub does not automatically redeliver failed deliveries."* Not an
   absent guarantee — a documented absence. (The contract doc's phrasing, *"do not guarantee … automatic
   retries"*, is weaker than reality and worth sharpening.)
2. **A delivery carries a queryable `status`.** *"if your server is down or takes longer than 10 seconds to
   respond, GitHub will record the delivery as a failure"*, and deliveries whose `status` is not `OK` count
   as failed.
3. **That status is the input to a documented recovery pattern.** GitHub publishes a sweep:
   `GET /orgs/{org}/hooks/{hook_id}/deliveries` → filter `status !== "OK"` → `POST
   /orgs/{org}/hooks/{hook_id}/deliveries/{delivery_id}/attempts`, consolidated by GUID to avoid duplicate
   replays. Their reference workflow runs **every 6 hours**; deliveries stay redeliverable for the **past
   3 days**.

**This resolves the question I had left open, and it resolves it against the current behaviour.** The status
code is not flow control, but it is not inert either: it is the single bit that decides whether a delivery
remains recoverable.

| Case | Response | Why |
| --- | --- | --- |
| Bad/missing signature | `401`/`403` | Must not be recorded as processed. |
| Event we deliberately ignore (`push`, `check_run`, …) | `204` | Nothing went wrong; the delivery genuinely succeeded. |
| Event we want but could not parse | **non-2xx** | Marks `status != OK`, which is the only thing that keeps the delivery visible to a redelivery sweep. |

Acking an unparseable payload with `204` stamps it `OK` and makes it **permanently unrecoverable** — no
sweep can ever find it, and GitHub discards it after 3 days. Replaying it later is impossible even once the
bug is fixed. That is a strictly worse outcome than a red line in the deliveries UI, and it is what the
code does today.

Note this reverses the rationale currently written into the handler. The inline comment in `src/main.rs`
justifies the `204` with "so GitHub does not retry a delivery that will fail the same way again": the
premise is false, and the conclusion it supports is the opposite of what the recovery path wants.

### 1.4 Canonical webhook path

**Verified**: four deliveries in the 2026-10-09 window hit `POST /api/v1/webhooks/github` and got `404`,
while the app serves `/github`. Some hook config disagrees with the app. Pick one path, fix the other end.

### 1.5 The 10-second budget

Missed in the first draft of this roadmap. *"Your server should respond with a 2XX response within 10
seconds… If your server takes longer than that to respond, then GitHub terminates the connection and
considers the delivery a failure."*

Currently irrelevant — observed latencies are 0–1ms, because the handler only parses and logs. It stops
being irrelevant the moment anything in the path does I/O. GitHub's guidance is explicit about the shape:
*"you may want to set up a queue to process webhook payloads asynchronously. Your server can respond when
it receives the webhook, and then process the payload in the background without blocking future webhook
deliveries."*

Interacts with 1.3: once processing is asynchronous, the response is sent **before** the outcome is known,
so a parse failure can no longer be signalled by the status code. Choosing async forecloses the recovery
path in 1.3 unless failures are tracked by us instead (3.2). Decide these two together, not separately.

## Phase 2 — the exporter

No metrics crate in `Cargo.toml` yet; `/metrics` does not exist. The contract doc's
*Prometheus cardinality warnings* section already settles the hard part — the allowed label set is
`repository`, `workflow_name`, `job_name`, `event`, `conclusion`, `status`, `runner_group_name`,
`runner_label`, and a parsed `ci_lib_version`; `runner_name`, `head_sha`, ids, `display_title`,
`head_branch`, and `steps[].name` are explicitly out.

### 2.1 Metric design

Open question, and the one with the most downstream consequence: **which actions emit.** Counting only
`completed` sidesteps the entire ordering problem (3.1) because a terminal event is self-contained. Gauges
of in-flight work need ordering-tolerant reconciliation and are strictly harder. Recommend starting
terminal-only.

### 2.2 The `started_at` trap

**Verified, and this will silently corrupt the headline metric.** On every `action: queued` job observed,
GitHub sets `started_at == created_at`:

| job | action | `created_at` | `started_at` |
| --- | --- | --- | --- |
| `build` (hosted) | `queued` | 17:34:38 | **17:34:38** |
| `build` (hosted) | `in_progress` | 17:34:38 | 17:34:40 |
| `renovate` (ARC) | `queued` | 17:34:51 | **17:34:51** |
| `renovate` (ARC) | `in_progress` | 17:34:51 | 17:34:54 |

`started_at` on a `queued` delivery is a placeholder, not a pickup time. `WorkflowJob::queue_wait()`
therefore returns `0` for every queued event. Real waits in that window were 2s hosted, 3s ARC.

**Only compute queue wait from `in_progress` or `completed`.** The method is currently dead code, so
nothing is wrong yet — but it is armed, and the contract doc calls this pair "the headline metric".

Same shape applies to `steps`: **verified** empty on `queued`, partial on `in_progress` (just
`Set up job`), complete on `completed`.

And `WorkflowRun::queue_delay()` is **verified** identically zero in production — `run_started_at ==
created_at` to the second on all three runs observed. The contract doc already says run-level is not a
runner-queue signal; live data agrees. Do not export it.

### 2.3 Hosted vs self-hosted

**Verified** discriminator that needs no label parsing: hosted runners report `runner_group_id: 0` /
`runner_group_name: "GitHub Actions"`; ARC reports group `6` / `"kubernetes"`.

## Phase 3 — resilience

### 3.1 Idempotency and ordering

Requirements, already in the contract doc: no ordering guarantee, duplicates possible, dedup on
`X-GitHub-Delivery` (stable across redelivery), reconcile on `(run_id, run_attempt, job id)`. Scope depends
entirely on 2.1 — terminal-only counters need far less of this than in-flight gauges.

**Verified** that interleaving is real, not theoretical: a `completed` for run `37967253110` arrived at
17:36:13, after run `37967342011` had already started and progressed.

### 3.2 Self-observability

A parse failure is currently an `ERROR` log and nothing else, and we ack it `204` (1.3). Needs a counter
for rejected and failed deliveries, or an org-wide metrics gap looks identical to an idle org.

### 3.3 Recovery — two mechanisms, not one

The contract doc names one (*"plan a periodic REST reconciliation"* via
`GET /repos/{owner}/{repo}/actions/runs`). GitHub documents a second, cheaper one that the first draft of
this roadmap missed entirely. They are not alternatives; they cover different failures.

| | Delivery redelivery sweep | Runs-API reconciliation |
| --- | --- | --- |
| Recovers | Deliveries GitHub attempted and we failed (`status != OK`) | Events never delivered at all, or older than the window |
| Payload | GitHub replays the **original body** — nothing to reconstruct | Rebuilt from the API; `referenced_workflows` is present, but the shape is not the webhook's |
| Window | **3 days** | Full history |
| Auth | PAT with `admin:org_hook` + `repo`, or a GitHub App — *"The built in `GITHUB_TOKEN` does not have sufficient permissions to redeliver webhooks."* | GitHub App installation token |
| Cost | Two endpoints, no payload mapping | HTTP client, App JWT → installation token exchange, refresh, plus a second mapping path to keep in sync |

**Do the sweep first.** It is a fraction of the work, it replays bodies we already know how to parse, and
it is the mechanism that 1.3's non-2xx decision exists to feed. Runs-API reconciliation stays on the list
for gaps outside the 3-day window, but it is the larger and later piece — it needs App auth and an entire
second deserialization path that has to agree with the webhook one.

Both need GitHub App auth eventually, which is the single largest unscoped dependency in this roadmap and
may warrant its own phase.

## Phase 4 — operations

### 4.1 Deployment

Nothing in the repo yet. `flake.nix` exists; no Dockerfile, no chart. We already run `infra-helm` and
`infra-charts`, so the shape is probably a chart there rather than anything new here.

### 4.2 CI hardening

`.github/workflows/rust.yml` runs `cargo build` and `cargo test` only. No `cargo clippy`, no
`cargo fmt --check`. Dead-code warnings already accumulate unchecked across the payload structs.

### 4.3 Event subscription scope

The contract doc specifies the **org-level** subscription so new repos need no per-repo setup, and
deliberately excludes `check_run` / `check_suite`. **Verified** that we currently receive `push`,
`check_run`, and `check_suite` as well — all correctly ignored, but they are wasted deliveries and
should be unsubscribed at the hook.

## Known gaps in what is modelled

| Gap | Status |
| --- | --- |
| `workflow_job`'s optional `deployment` field | Not modelled; silently dropped. Only matters if we run deployment jobs. |
| `workflow_job` action `waiting` | Modelled, never observed live. |
| Real envelope fixtures | `tests/fixtures/` holds only the **inner** `workflow_run` object. Envelope tests reconstruct a body from it; no captured full delivery exists. The contract doc's own advice — diff a real delivery against the schema — is not yet possible for the envelope. |
| `workflow` field optionality | Typed `Option<Workflow>` in code; the contract doc lists `workflow` as **required** for `workflow_run`. Code is looser than contract. Reconcile. |
