# Long-running agent harness – reference

Templates and conventions for harness artifacts. Use when creating or refreshing the harness in a new project.

**Flow**: Design doc (human + AI) → Initializer runs once (reads design doc, creates feature list, runbook, backlog, progress) → Coding agent runs in a loop. At milestones (e.g. end of a work-unit group), pause for human check-in; when satisfied, re-run Initializer then continue until all features pass. The top-level agent spawns sub-agents (using whatever mechanism Cursor provides) and passes the relevant prompt each time (no copying into `.cursor/rules/`).

## Directory layout (canonical)

All harness artifacts live in **`_working/`**. Paths are relative to project root.

```
<project-root>/
└── _working/
    ├── design.md                         # Design doc (human + AI); input to Initializer
    ├── agent-progress.md                 # Session log, state, next target (Coding Agent updates)
    ├── agent-runbook.md                  # How to run/verify (Initializer + human)
    ├── implementation-backlog.md         # Work units for Coding Agent to pick from
    ├── feature_list.yaml                 # Acceptance criteria in YAML; passes true/false
    └── init_<app>.sh                     # Optional one-command start (e.g. init_myapp.sh)
```

---

## Artifact templates

Initializer writes these into `_working/`. Template content (same filenames in this folder):

| File                                                                     | Description                                                         |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| [agent-runbook.md](agent-runbook.md)                   | Runbook: start app, lint/typecheck, tests.                          |
| [implementation-backlog.md](implementation-backlog.md) | Backlog: one entry per work unit; derive from design doc.            |
| [agent-progress.md](agent-progress.md)                 | Progress: Initializer seeds; Coding Agent appends Session N blocks. |

---

## feature_list.yaml schema

- **acceptance_tests**: list of objects.
- Each object: **description** (string), **category** (string), **passes** (boolean), **steps** (list of strings).
- Categories can be e.g. `auth`, `functional`, `data-display`, `edge-case`, `error-case`.
- Order by implementation dependency (e.g. auth first). Initializer creates all with `passes: false`; Coding Agent sets `passes: true` only after verification.

---

## Prompt templates (for sub-agents)

The top-level agent uses these when spawning sub-agents; it passes the relevant prompt content each time. All paths use `_working/`.
| File | Use as |
|------|--------|
| [initializer-prompt.md](initializer-prompt.md) | Prompt for the Initializer sub-agent. Reads `_working/design.md`; writes `_working/feature_list.yaml`, `_working/agent-runbook.md`, `_working/implementation-backlog.md`, `_working/agent-progress.md`, optionally `_working/init_<app>.sh`. Does not implement features or create project scaffolding. |
| [coding-prompt.md](coding-prompt.md) | Prompt for each Coding sub-agent session. Reads/writes `_working/` artifacts; verifies using the runbook only. At milestones, the top-level agent pauses the loop, checks in with the human, then re-runs the Initializer and resumes the loop. |

