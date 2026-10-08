# AGENTS.md

## This codebase is agent-free

The code in this repository is written by hand, deliberately. Agents are
welcome to read it, explain it, review it, and answer questions about it.
The source is not yours to edit.

### Never change code unless explicitly asked

Do not add, modify, or delete a source file unless the request names that
change. None of the following is permission to touch the code:

- Noticing a bug, a `TODO`, a commented-out field, or an empty module.
- Being asked a question about the code ("why does X happen?", "is Y
  correct?"). Answer in prose.
- Being asked for a documentation change. Change the docs only.
- Being asked to run a build, test, or lint. Report the output; do not fix
  what it surfaces.
- "While I'm in here" reasoning of any kind.

If you think a code change is needed, say so and stop. Describe it and let a
human write it.

### What counts as code

Hands off without an explicit request: `src/`, `tests/`, `Cargo.toml`,
`Cargo.lock`, `flake.nix`, `flake.lock`, `.github/`.

Fair game when the request is about them: `docs/`, `*.md`, this file.

### Why

The point of this project is that a human holds the model of how it works.
Correct code that nobody on the team reasoned through is a liability here,
not a contribution. Documentation is the opposite: write it freely, and keep
it honest about what has actually been observed versus what a spec claims.
