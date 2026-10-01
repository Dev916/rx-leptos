# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/) once the crate is published.

## [Unreleased]

### Added

- `from_signal` / `SignalExt::to_observable`: a signal, memo or any `Get`
  type as a `Local` observable, with the current value on subscribe and each
  change on the next executor tick (writes within a tick coalesce).
- `to_signal`, `to_signal_local`, `use_observable` and the method forms
  `ObservableExt::{to_signal, to_signal_local, to_option_signal}`: an
  observable as a read signal owned by the current reactive owner.
- `to_memo` / `ObservableExt::to_memo`: an observable as a `Memo<T>`, so equal
  consecutive items do not notify dependents.
- `feed_signal` / `ObservableExt::feed`: write every item into an existing
  `WriteSignal` until the owner is cleaned up; the server-rendering pattern.
- `from_resource` / `ResourceExt::resolved`: the resolved values of a
  `Resource`, `LocalResource` or any `Get<Value = Option<T>>`, skipping the
  pending `None`.
- `use_subject` and `use_subscription`: a `Subject` that completes, and a
  subscription that ends, when the current reactive owner is cleaned up.
- wasm only: `from_event`, `animation_frames`, `from_fetch` and
  `from_fetch_with`, each releasing its browser resource on unsubscribe.
- `prelude` with the bridges, their method forms and the `reactive_graph`
  access traits; `reactive_graph` and `rxrust` re-exported.
- Examples: `examples/leptos-csr` (client-side, deployed to GitHub Pages) and
  `examples/leptos-ssr` (server-side with hydration, deployed as a Cloudflare
  Container), with a Playwright suite in `e2e/`.

### Notes

- Depends on the [Dev916/rxRust](https://github.com/Dev916/rxRust) fork as a
  git dependency until its operator work is published; the crate is therefore
  not yet on crates.io.
