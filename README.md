![Ardite Logo](logo.png)

# Ardite Core

[![Build Status][1]][2]
[![Clippy Linting Result][3]][4]

[1]: https://travis-ci.org/ardite/ardite-core.svg?branch=master
[2]: https://travis-ci.org/ardite/ardite-core
[3]: https://clippy.bashy.io/github/ardite/ardite-core/develop/badge.svg
[4]: https://clippy.bashy.io/github/ardite/ardite-core/develop/log

Ardite provides the API design *you* want from the database of *your* choice. Ardite provides a more standard compliant, flexible, stable, secure, and faster API then any you might write in house.

This package provides core interfaces to connect drivers with the user facing binaries. To get started with Ardite, we need code reviews! Start with `src/value.rs` and `src/query.rs` where most of the work is currently being done. If you see something noteworthy open an issue.