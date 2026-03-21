# 🍪 `git-template`

*Create new repositories from existing repositories, with interpolation support.*

<!-- rumdl-disable MD013 -->
[![CI](https://github.com/git-ents/git-template/actions/workflows/CI.yml/badge.svg)](https://github.com/git-ents/git-template/actions/workflows/CI.yml)
[![CD](https://github.com/git-ents/git-template/actions/workflows/CD.yml/badge.svg)](https://github.com/git-ents/git-template/actions/workflows/CD.yml)
<!-- rumdl-enable MD013 -->

> [!CAUTION]
> This project is in active development and has not yet been published to crates.io.
> Please file a [new issue] for any misbehaviors you find!

[new issue]: https://github.com/git-ents/git-template/issues/new

## Overview

This crate is the top-level entry point for the `git-template` workspace.
It wires together the domain crates — issues, reviews, and releases — into a single `git template` CLI and re-exports them as a unified library facade.

## Installation

### CLI

The `git-template` command can be installed with `cargo install`.

```shell
cargo install --locked --git https://github.com/git-ents/git-template.git git-template
```

If `~/.cargo/bin` is on your `PATH`, you can invoke the command with `git`.

```shell
git template -h
```

### Library

The `git-template` library can be added to your Rust project via `cargo add`.

```shell
cargo add --git https://github.com/git-ents/git-template.git git-template
```
