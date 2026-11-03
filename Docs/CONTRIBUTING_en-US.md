[简体中文](../CONTRIBUTING.md) | [繁體中文](CONTRIBUTING_zh-HK.md) | **English**

# Contributing Guide

Thank you for your interest in Floral Notepaper! This guide will help you get started with contributing to the project.

## Development Environment Setup

### Dependencies

- [Node.js](https://nodejs.org/) - 22.12+ (required by Mermaid 12; Node 24 LTS recommended)
- [Rust](https://www.rust-lang.org/tools/install) toolchain
- Additional:

  | Platform    | Required Dependencies                                                                                                                                                                                                                      |
  | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
  | **macOS**   | Xcode Command Line Tools                                                                                                                                                                                                                   |
  | **Linux**   | `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libsoup-3.0-dev`                                                                                                                                 |
  | **Windows** | [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe), [Microsoft Edge WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (pre-installed on Windows 11 and later versions of Windows 10) |

### Clone & Launch

```bash
git clone git@github.com:Achilng/floral-notepaper.git
cd floral-notepaper
npm install

# Start development build
npm run tauri dev

# Build for production
npm run tauri build
```

## Branch & Commit Conventions

This project follows the [Conventional Commits](https://www.conventionalcommits.org/) specification.

### Commit Message Format

```
<type>: <brief description>
```

Common types:

| type       | Description                                    |
| ---------- | ---------------------------------------------- |
| `feat`     | New feature                                    |
| `fix`      | Bug fix                                        |
| `docs`     | Documentation changes                          |
| `style`    | Formatting (no logic changes)                  |
| `refactor` | Code refactoring (neither new feature nor fix) |
| `perf`     | Performance optimization                       |
| `test`     | Adding or modifying tests                      |
| `chore`    | Build, CI, dependencies, and other miscellanea |
| `ci`       | CI/CD configuration changes                    |
| `revert`   | Revert a commit                                |

### Branch Naming

Branch names should correspond to the commit type: `feat/description`, `fix/description`, `refactor/description`, etc.

Examples:

- `feat/note-tagging` → new feature branch
- `fix/startup-crash` → bug fix branch

## Code Style & Testing

Before submitting a PR, ensure all local checks pass. It's a good idea to run them before each commit as well to avoid CI failures:

```bash
# Backend
cd src-tauri
cargo test -p floral-notepaper --lib        # Unit tests
cargo clippy --all-targets -- -D warnings   # Lint
cargo fmt --check                           # Format check

# Frontend
npm test                                    # Unit tests
npx oxlint                                  # Lint
npx oxfmt --check                           # Format check
```

## Pull Request Process

1. **Search** [existing PRs](https://github.com/Achilng/floral-notepaper/pulls) first to avoid duplicate work
2. Fork this repository and create your feature/fix branch from `main`
3. Once development is complete, ensure all local checks pass
4. Submit a PR to the `main` branch, filling out all sections of the **PR template**
5. CI must pass (build + lint + tests)

   > Note: CI runs against the merged result of your PR branch and `main` (`refs/pull/<number>/merge`), not your branch's original commits.

6. At least one maintainer must **approve the review** before merging
7. Maintainers use **squash merge**; remote branches are automatically deleted after merging

PR titles should also follow Conventional Commits, e.g. `feat: add note tagging feature`.

## Feedback

- Report a bug: [Submit an Issue](https://github.com/Achilng/floral-notepaper/issues/new?template=bug.yml)
- Feature request: [Submit an Issue](https://github.com/Achilng/floral-notepaper/issues/new?template=enhancement.yml)
- General discussion: [GitHub Discussions](https://github.com/Achilng/floral-notepaper/discussions)
