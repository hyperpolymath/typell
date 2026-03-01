# SPDX-License-Identifier: PMPL-1.0-or-later

# integrations/ — Consumer Integrations

Typell's verification kernel is consumed by multiple tools through the
Verification Protocol (JSON-RPC). This directory contains the integration
code for each consumer.

## Structure

```
integrations/
├── panll/    # PanLL integration (PRIMARY consumer)
│             Typell as PanLL's Pane-N reasoning engine.
│             Can run as: Tauri plugin (in-process) or JSON-RPC server.
│             Provides: type feedback for Pane-L, reasoning for Pane-N,
│             validated results for Pane-W.
│
├── vscode/   # VS Code extension (secondary consumer)
│             LSP-like diagnostics, completions, hover info.
│             For developers who want type feedback without full PanLL.
│
├── cli/      # Command-line interface (secondary consumer)
│             Query validation at the terminal.
│             Used by CI/CD pipelines and scripts.
│
└── ci/       # CI/CD plugins (secondary consumer)
              GitHub Actions, GitLab CI integration.
              Automated proof checking in pipelines.
```

## Priority

PanLL is the primary consumer. All other integrations are secondary and must
not influence architectural decisions that would complicate PanLL integration.
