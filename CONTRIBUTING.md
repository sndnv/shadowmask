# Contribution Guidelines

The easiest way to get started with questions, bugfixes or feature requests is to open an issue, or a pull request.

> *This is not a rulebook to be followed strictly, but a general guide to what might make sense most of the time.*

## Issues, Commits, Commit Messages, and PRs

Ideally, an issue would be created before doing any major work on a new feature/bugfix. This serves the purpose of
getting a discussion going as early as possible and allows for tracking related changes by including the issue number in
the commit messages.

Short and descriptive commit messages are preferred over detailed explanations of the changes; if more context is
needed, that information should be in the pull request.

## Code Style

Use of code style and linting tools is encouraged; this will help with code consistency and reduces time spent
discussing style or re-doing work. Formatting (`rustfmt`) and linting (`clippy`) are enforced automatically via
[`qa.py`](qa.py) and in CI.

Most IDEs are capable of assisting with this task as well; for example, [IntelliJ](https://www.jetbrains.com/idea/) or
[VS Code](https://code.visualstudio.com/) with `rust-analyzer` can show warnings and re-format code.

In general, style warnings/warts should be suppressed only if there is no reasonable way of avoiding them.

## QA and Testing

To maintain the quality of the codebase, extensive testing is essential. All checks - formatting, linting, build,
tests and coverage - run through a single script, [`qa.py`](qa.py), both locally and in CI.

Smaller and simpler test scenarios, with limited setup/teardown, are encouraged; this approach tends to create
components that are relatively small, modular and with a limited set of responsibilities. The external boundaries
(FFmpeg and SQLite) are mocked, and unit coverage is held at 100% via [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov).
