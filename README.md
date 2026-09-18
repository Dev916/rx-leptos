# rx-leptos

Bridges between [rxRust](https://docs.rs/rxrust) observables and Leptos 0.8
signals, built on `reactive_graph` (the reactive core Leptos re-exports).

| Function | Direction | Notes |
|----------|-----------|-------|
| `from_signal(signal)` | signal → observable | Emits the current value on subscribe, then every change on the next executor tick (writes within a tick coalesce). Works with signals, memos and anything else implementing `Get`. No `effects` feature needed. |
| `to_signal(obs, initial)` / `to_signal_local` | observable → signal | The signal and the subscription belong to the current reactive owner and are disposed with it. |
| `use_observable(obs)` | observable → `ReadSignal<Option<T>>` | `None` until the first item. |
| `from_event(target, "click")` | DOM → observable | wasm only; removes the listener on unsubscribe. |
| `animation_frames()` | `requestAnimationFrame` → observable | wasm only; `{ timestamp, elapsed }` per frame, cancelled on unsubscribe. |
| `from_fetch(url)` / `from_fetch_with(url, init)` | `fetch` → observable | wasm only; one `Response` then completion, `JsValue` error, aborted on unsubscribe. |
| `feed_signal(obs, write)` | observable → existing signal | Writes every item into a `WriteSignal` until the owner is cleaned up. The SSR-friendly form: create the signal at component level, call this inside `Effect::new`. |
| `use_subject::<T>()` | owner-scoped `Subject` | Completes (and releases subscribers) when the owner is cleaned up. Feed it from event handlers. |
| `use_subscription(sub)` | owner-scoped subscription | Unsubscribes when the owner is cleaned up; the rx counterpart of `Effect::new` for side effects. |

Method forms come from `SignalExt` and `ObservableExt` in the prelude:

```rust
let results = query
  .to_observable()
  .debounce(Duration::from_millis(300))
  .distinct_until_changed()
  .switch_map(search)
  .to_signal(Vec::new());
```

`rx_leptos::reactive_graph` and `rx_leptos::rxrust` re-export the underlying
crates; `leptos::prelude` exports the same `reactive_graph` types, so
glob-importing both preludes is fine.

Everything uses rxRust's `Local` context: signals are single-threaded UI state.

## Server-side rendering

Leptos's server integrations have no tokio `LocalSet`, so `spawn_local`
panics during server render; rxRust's `LocalScheduler` timers and
`from_signal` change delivery both need it. Create signals at component level
and wire pipelines inside `Effect::new` (browser only), feeding the signals
with `feed_signal` / `.feed(write)`. [`examples/leptos-ssr`](examples/leptos-ssr)
shows the pattern with `cargo leptos`.

## Outside Leptos

Leptos initialises the global `any_spawner` executor when it mounts or serves an
app. In plain tests, do it yourself:

```rust
let _ = any_spawner::Executor::init_futures_executor();
// ... write signals ...
any_spawner::Executor::poll_local(); // deliver pending changes
```

## Example

[`examples/leptos-csr`](examples/leptos-csr) is a Leptos 0.8 client-side
app built on these bridges; run it with `trunk serve`.

## Status

Pre-release, not yet on crates.io; the API may still change. Depends on the
[Dev916/rxRust](https://github.com/Dev916/rxRust) fork (git dependency) until
its operator work is published.

## Live demos

| App | URL | How it runs |
|-----|-----|-------------|
| CSR example | <https://dev916.github.io/rx-leptos/> | static `trunk` build on GitHub Pages, deployed from `master` by `pages.yml` |
| SSR example | <https://rx-leptos-ssr.webmech.workers.dev> | the `Dockerfile` image on Cloudflare Containers behind a Worker (`deploy/cloudflare`), deployed by `cloudflare.yml` |

The Playwright suite in `e2e/` gates every PR against a local build and runs
again against each live deployment after it goes out. Locally,
`BASE_URL=<url>/ npx playwright test` points it anywhere.

### Deploying the SSR container yourself

```sh
docker buildx build --platform linux/amd64 -t rx-leptos-ssr -f Dockerfile .   # optional local check
cd deploy/cloudflare && npm ci
CLOUDFLARE_API_TOKEN=... npx wrangler deploy      # builds the image, pushes it, deploys the Worker
```

CI does the same from `master` once the `CLOUDFLARE_API_TOKEN` repository
secret exists (the account id is pinned in `deploy/cloudflare/wrangler.jsonc`).

## Development

```sh
cargo test --workspace                 # crate + CSR example models
wasm-pack test --node                  # DOM/web sources in node
cd examples/leptos-csr && trunk serve  # browser
(cd examples/leptos-csr && trunk build --release) && (cd e2e && npm ci && npx playwright test)  # browser tests
cd examples/leptos-csr && trunk build --release && (cd ../../e2e && npm ci && npx playwright test)
cd examples/leptos-ssr && cargo leptos serve
```
