# Repository Structure

This document describes the current repository layout.

```
.
├── .github/
│   └── workflows/
│       └── codex-issue-to-pr.yml
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── README.md
└── src/
    └── main.rs
```

## Notes

- `.github/workflows/codex-issue-to-pr.yml`: GitHub Actions workflow.
- `Cargo.toml`: Rust package manifest.
- `Cargo.lock`: Locked dependency versions.
- `README.md`: Project overview and usage.
- `src/main.rs`: Application entry point.
