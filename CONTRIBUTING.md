# Contributing to m8s

Thank you for considering contributing to `m8s`! Here are some guidelines to help you get started.

## Setting Up the Development Environment

1. **Install Rust**: Follow the instructions on [rust-lang.org](https://www.rust-lang.org/).
2. **Clone the Repository**:
    ```shell
    git clone https://github.com/conradkleinespel/m8s.git
    cd m8s
    ```
3. **Set Up Git Hooks**:
    ```shell
    git config --local core.hooksPath githooks/
    ```
4. **Run the Project Locally**:
    ```shell
    cargo run
    ```

## Coding Standards

- Follow the Rust coding conventions.
- Ensure your code is formatted by running `cargo fmt`.
- Run `cargo clippy` to catch common mistakes and improve your code.

## Submitting Pull Requests

1. Fork the repository and create your branch from `main`.
2. If you've added code for new features or bug fixes, add corresponding tests.
3. Ensure the test suite passes by running `cargo test`.
4. Submit a pull request with a clear description of your changes.

## Reporting Issues

If you find a bug or have a feature request, please open an issue on GitHub.

Thank you for your contributions!
