# GitHub Actions webhook contract

Source of truth for the payloads this exporter ingests.

- **Schema**: `@octokit/webhooks-schemas@7.6.1` (`https://unpkg.com/@octokit/webhooks-schemas@7.6.1/schema.json`) — GitHub's
  published webhook schemas, machine-readable.
- **Verified against live data**: `WMS-DEV/infra-charts` run `37610094675` and its jobs, 2026-10-07.
  Everything marked *verified* below was observed in our own org, not read off a doc page.

## Events we subscribe to

| Event | Actions | Why |
| --- | --- | --- |
| `workflow_run` | `requested`, `in_progress`, `completed` | Run-level outcome, duration, and **reusable-workflow version attribution** |
| `workflow_job` | `queued`, `in_progress`, `completed`, `waiting` | Per-job outcome, **queue wait on ARC runners**, runner identity |

Permission required (GitHub App): *"A GitHub App must have at least read-level access for the 'Actions'
repository permission."* Subscribable at repository, organization, business, and app level — take the
**org-level** subscription so new repos are covered without per-repo setup.

`check_run` / `check_suite` are deliberately excluded: they duplicate workflow state at higher volume and
carry no Actions-specific fields we need.

## Reusable workflows — the part that actually matters here

`infra-ci-lib` is entirely `workflow_call` reusable workflows. Consequences, all verified:

1. **A called reusable workflow does NOT produce its own `workflow_run`.** There is exactly one run, owned
   by the *caller* repo. Counting runs per workflow will never show `infra-ci-lib` workflows directly.
2. **`workflow_run.referenced_workflows[]`** is how you attribute to `infra-ci-lib`. Observed:
   ```json
   "referenced_workflows": [{
     "path": "WMS-DEV/infra-ci-lib/.github/workflows/secret-scanning.yml@v1.1.2",
     "sha":  "7583bfa4f0f3e0ce054ffe4eb3c245d4bd5bd631",
     "ref":  "refs/tags/v1.1.2"
   }]
   ```
   `path` embeds `owner/repo/path@ref`, so the pinned version is parseable straight out of it, and `sha` is
   the resolved commit. This gives **version-drift tracking across the org for free** — which repos are still
   on `v1.1.2` while `v1.2.1` is out. `path` and `sha` are required; `ref` is optional (absent for a raw-SHA pin).
3. **`workflow_job.name` is prefixed with the caller's job key**: observed `"scan / Gitleaks"` — i.e.
   `<caller job id> / <job name inside the reusable workflow>`. Split on `" / "` if you want the inner name.
4. **`workflow_job.workflow_name` is the CALLER's workflow name** (`"Secret Scanning"`), not the reusable
   workflow's. Do not use it to identify an `infra-ci-lib` workflow.
5. Note the caller files are `.yaml` while `infra-ci-lib` uses `.yml`; don't key anything on extension.

## `workflow_run` payload

Top-level keys: `action`, `workflow_run`, `workflow`, `repository`, `organization`, `sender`, `installation`.
Only `action`, `workflow_run`, `workflow`, `repository`, `sender` are required — see
[Generic objects](#generic-objects).

`workflow_run` (all required unless noted):

| Field | Type | Notes |
| --- | --- | --- |
| `id` | integer | Run identity |
| `run_number` | integer | Per-workflow monotonic counter |
| `run_attempt` | integer | **Increments on re-run** — key dedup/identity with this |
| `workflow_id` | integer | Stable per workflow definition |
| `name` | string | Workflow name |
| `path` | string | e.g. `.github/workflows/secret-scanning.yaml` |
| `display_title` | string | Commit/PR title |
| `event` | string | Trigger: `push`, `pull_request`, `schedule`, `workflow_dispatch`, … |
| `status` | enum | `requested`, `in_progress`, `completed`, `queued`, `waiting` |
| `conclusion` | enum \| null | `success`, `failure`, `neutral`, `cancelled`, `timed_out`, `action_required`, `stale`, `skipped`; **null until `completed`** |
| `created_at` | date-time | Run created |
| `run_started_at` | date-time | When GitHub *accepted* the run, not when a runner picked it up. Verified equal to `created_at` on a run whose job waited 20s, so this is **not** a runner-queue signal — use the job-level pair instead |
| `updated_at` | date-time | |
| `head_branch` | string | |
| `head_sha` | string | |
| `head_commit` | object | [`commit-simple`](#generic-objects) |
| `repository`, `head_repository` | object | [`repository-lite`](#generic-objects) — **not** the full top-level `repository` |
| `actor`, `triggering_actor` | object | [`user`](#generic-objects); differ on re-runs |
| `pull_requests` | array | Stub objects only — [see below](#pull_requests-and-repo-ref) |
| `referenced_workflows` | array | **optional** — see above |
| `previous_attempt_url` | string \| null | Set when `run_attempt > 1` |
| `check_suite_id`, `check_suite_node_id` | integer, string | |
| `*_url` | uri | `html_url`, `jobs_url`, `logs_url`, `artifacts_url`, `cancel_url`, `rerun_url`, `workflow_url`, `check_suite_url`, `url` |

`conclusion` is narrowed to non-null **only** in the `completed` variant. `requested` and `in_progress`
carry no overrides — treat every field as nullable until you've matched on `action`.

The sibling `workflow` object: `id`, `node_id`, `name`, `path`, `state`, `created_at`, `updated_at`,
`url`, `html_url`, `badge_url`.

## `workflow_job` payload

Top-level keys: `action`, `workflow_job`, `repository`, `organization`, `sender`, `installation`,
`deployment` (optional). Only `action`, `workflow_job`, `repository`, `sender` are required — see
[Generic objects](#generic-objects).

| Field | Type | Notes |
| --- | --- | --- |
| `id` | integer | Job identity |
| `run_id` | number | Joins to `workflow_run.id` |
| `run_attempt` | integer | |
| `name` | string | `"scan / Gitleaks"` for reusable-workflow jobs |
| `workflow_name` | string \| null | Caller's workflow name |
| `head_branch` | string \| null | |
| `head_sha` | string | |
| `status` | enum | `queued`, `in_progress`, `completed`, `waiting` |
| `conclusion` | enum \| null | `success`, `failure`, `cancelled`, `skipped`; **null until `completed`** |
| `created_at` | date-time | Job created / enqueued |
| `started_at` | date-time | Picked up by a runner |
| `completed_at` | date-time \| null | **Non-null only on `action: completed`** |
| `labels` | string[] | `runs-on` labels — observed `["github-arc-runner"]` |
| `runner_id` | integer \| null | |
| `runner_name` | string \| null | **Ephemeral ARC pod name** — observed `github-arc-runner-fxjbf-runner-bvk9h` |
| `runner_group_id` | integer \| null | |
| `runner_group_name` | string \| null | Observed `"kubernetes"` |
| `steps` | array | See below |
| `url`, `html_url`, `run_url`, `check_run_url` | uri | |

**`created_at` → `started_at` is the ARC queue wait** — 20s on the verified run. This is the headline
metric for self-hosted runner saturation and the main reason to ingest `workflow_job` at all.

### `steps[]` (completed variant)

`name` (string), `number` (integer), `status` (`completed`), `conclusion`
(`success` \| `failure` \| `skipped` \| `cancelled`), `started_at`, `completed_at`.

In-progress and queued steps have correspondingly narrower shapes; `status` is `queued` / `in_progress`
with null timestamps. Per-step metrics are a cardinality trap — see below.

## Generic objects

Both payloads are mostly made of shared definitions that the tables above name but do not spell out
(`commit-simple`, `repository-lite`, `user`, …). They are reused verbatim across every GitHub webhook, so
they are documented once, here. The tables below list the non-URL fields; the `*_url` fields are elided for
readability only — they are uniformly `string` and required, several are URI *templates* (`{/other_user}`,
`{archive_format}`), and the one exception is `organization.html_url`, which is **optional**.

### Which object sits where

| Location | Definition |
| --- | --- |
| top-level `repository` | `repository` — the **full** object |
| `workflow_run.repository`, `workflow_run.head_repository` | `repository-lite` |
| top-level `sender`, `workflow_run.actor`, `workflow_run.triggering_actor`, `repository*.owner` | `user` |
| top-level `organization` | `organization` |
| top-level `installation` | `installation-lite` |
| `workflow_run.head_commit` | `commit-simple` |
| `workflow_run.head_commit.author`, `.committer` | `committer` |
| `workflow_run.pull_requests[].head.repo`, `.base.repo` | `repo-ref` |

**The two `repository` shapes are not the same object.** The top level carries the full definition — 96
extra keys including `default_branch`, `visibility`, `archived`, `topics`, `pushed_at`, `language`,
`custom_properties`, and the counters. Nested under `workflow_run` you get `repository-lite`, which has
none of them. A type that deserializes one will reject the other if it requires any full-only field, so
either model them separately or model only the lite subset and read it from both positions.

**`organization` and `installation` are optional at the top level.** The required set for both
`workflow_run$*` and `workflow_job$*` is only `action`, `repository`, `sender`, and the event object
(plus `workflow` for `workflow_run`). Org-level subscriptions do send `organization`, but it is not
contractually there — do not make it a hard dependency of parsing. Same for `installation`, which is
absent outside GitHub App deliveries.

### `repository-lite`

| Field | Type | Notes |
| --- | --- | --- |
| `id` | integer | |
| `node_id` | string | |
| `name` | string | `infra-charts` |
| `full_name` | string | `WMS-DEV/infra-charts` — **this is the `repository` metric label** |
| `private` | boolean | verified `true` |
| `fork` | boolean | |
| `description` | string \| null | |
| `owner` | object | `user` |

All required, `description` nullable. 38 URL fields besides.

### `user`

| Field | Type | Notes |
| --- | --- | --- |
| `login` | string | |
| `id` | integer | |
| `node_id` | string | |
| `type` | enum | `Bot` \| `User` \| `Organization` |
| `site_admin` | boolean | |
| `gravatar_id` | string | required, verified empty `""` |
| `name` | string | **optional** |
| `email` | string \| null | **optional** |

`type` is the useful one: it separates automation from humans without parsing logins. Verified — a Renovate
app arrives as `actor.type: "Bot"` with `login: "…[bot]"`, while `repository.owner.type` is `"Organization"`.
The `[bot]` suffix is a convention; `type` is the contract.

### `commit-simple` and `committer`

`commit-simple`: `id` (string — the commit SHA, *not* an integer), `tree_id` (string), `message` (string),
`timestamp` (date-time), `author` and `committer` (both `committer` objects). All required.

`committer`: `name` (string, required), `email` (string \| null, required), `date` (date-time, optional),
`username` (string, optional). On the verified run only `name` and `email` were present — treat `date` and
`username` as genuinely absent, not null.

`message` is free text and `id` is unbounded: both are in the cardinality table below for a reason.

### `organization` and `installation-lite`

`organization`: `login`, `id`, `node_id`, `description` (string \| null), all required, plus 9 URL fields.
`login` is the org slug and the only field worth keeping.

`installation-lite`: `id` and `node_id`, both required. Nothing else — if you need installation detail you
have to call the API.

### `pull_requests[]` and `repo-ref`

Each element: `url`, `id` (number), `number` (number), `head`, `base` — all required,
`additionalProperties: false`. `head` and `base` are each `{ ref, sha, repo }`, all required, where `repo`
is a `repo-ref`: `{ id, url, name }`, all required, `additionalProperties: false`.

Note what is *not* here: no `title`, no `state`, no `merged`. This is a stub, not `pull-request`. Verified —
`pull_requests[0].number` was `60` with `head.ref: "renovate/amazon-aws-cli-2.x"` and `base.ref: "master"`,
and that is the whole of what a `workflow_run` tells you about the PR. `base.ref` is the one genuinely
useful field, since it is the bounded branch (`master`/`main`) that `head_branch` is not.

The array is required but empty for non-PR events.

### Live payloads carry fields the schema does not

Verified: every `user` in our payloads has a `user_view_type` (`"public"`) that
`@octokit/webhooks-schemas@7.6.1` does not define. GitHub adds fields without a schema release.

Consequence: **never reject unknown fields.** Deserialization must ignore them, or a routine GitHub-side
addition becomes a parse failure and a silent metrics gap across the whole org. The schema is a floor on
what arrives, not a description of it.

## Delivery semantics

GitHub's webhook docs **do not guarantee** ordering, at-least-once delivery, or automatic retries. What is
documented: the `X-GitHub-Delivery` header is unique per delivery, and *"if you request a redelivery, the
`X-GitHub-Delivery` header will be the same as in the original delivery."*

Design consequences — these are requirements, not nice-to-haves:

- **No ordering.** A `completed` can arrive before the matching `in_progress`. Never implement state as a
  transition machine that assumes arrival order; reconcile on `(run_id, run_attempt, job id)` and let the
  payload's own `status`/`conclusion` plus timestamps decide the terminal state.
- **Duplicates are possible** (including operator-triggered redeliveries). Every handler must be idempotent.
  Dedup on `X-GitHub-Delivery`, which is stable across redelivery — exactly the behaviour you want for
  idempotency, even though it means the header can't tell you *that* it was a redelivery.
- **Gaps are possible.** Webhooks alone will drift. Plan a periodic REST reconciliation
  (`GET /repos/{owner}/{repo}/actions/runs`) to backfill missed deliveries; the same endpoint returns
  `referenced_workflows`, so it's a drop-in backfill for the webhook path.
- Verify `X-Hub-Signature-256` (HMAC-SHA256) before parsing anything.

## Prometheus cardinality warnings

Straight from the verified payloads — do **not** make these labels:

| Field | Why |
| --- | --- |
| `runner_name` | Ephemeral ARC pod, unique per job. Unbounded. |
| `head_sha`, `id`, `run_id` | Unbounded by construction. |
| `display_title`, commit messages | Free text. |
| `head_branch` | Unbounded on PR-heavy repos — safe only if bucketed to `main`/`master`/`other`. |
| `steps[].name` | Unbounded across the org; step-level belongs in logs/traces, not metric labels. |

Reasonable label set: `repository`, `workflow_name`, `job_name`, `event`, `conclusion`, `status`,
`runner_group_name`, `runner_label`, and a parsed `ci_lib_version` from `referenced_workflows`.

## Regenerating

```sh
curl -sL https://unpkg.com/@octokit/webhooks-schemas@7.6.1/schema.json -o schema.json
jq -r '.definitions | keys[] | select(test("workflow_job|workflow_run"))' schema.json
```

Per-action variants live at `.definitions["workflow_job$completed"]` etc. and narrow the base
`workflow-job` / `workflow-run` definitions — always read both.

The generic objects above come out of the same file. To re-derive one, minus the URL noise:

```sh
jq -r '.definitions["repository-lite"] as $o
  | ($o.required // []) as $req
  | $o.properties | to_entries[]
  | select(.key | test("_url$|^url$") | not)
  | "\(.key): \(.value["$ref"] // (.value.type | tostring))\(if (.key | IN($req[])) then "" else " [optional]" end)"
' schema.json
```

Live payloads are the other half of the check — the schema omits fields GitHub actually sends (see above),
so diff a real delivery against it rather than trusting either alone.
