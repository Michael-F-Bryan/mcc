# Agent runbook – [Project name]

Single source of truth for "how do I run and verify the app?". Use these commands; do not guess.

## Start app

- **Backend**. Working directory: **[path]**.
  ```bash
  [exact command]
  ```
  Default: [URL if any].

- **Frontend** (or other client). Working directory: **[path]**.
  ```bash
  [exact command]
  ```
  Env: [.env file or vars].

## Lint / typecheck

- **[Component]**. Working directory: **[path]**.
  ```bash
  [exact command]
  ```

## Tests

- **Unit/component**. Working directory: **[path]**.
  ```bash
  [exact command]
  ```

- **E2E / acceptance**. Working directory: **[path]**. Requires [app running / DB / etc.].
  ```bash
  [exact command]
  ```
  Acceptance criteria are in `_working/feature_list.yaml`; the Coding agent sets `passes: true` only after running the verification below.

## Verifying acceptance criteria

Document **how** to verify each type of acceptance test so the Coding agent can run these steps. Include exact commands, tools, and—if the project has a UI or browser flows—how to run browser automation or manual checks (e.g. which tool, which URL, key steps). The Coding agent will use this as the single source of truth; do not leave verification to guesswork.

- **Unit/integration**: [commands or test names]
- **E2E / UI / browser** (if applicable): [e.g. Playwright/Cypress command, or manual checklist with URLs and steps]

## Optional: one-time setup

- [Install deps, migrate DB, seed data – exact commands.]
