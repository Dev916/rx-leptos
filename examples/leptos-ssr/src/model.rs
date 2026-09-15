//! View-independent reactive models, structured for server rendering.
//!
//! Each model creates its signals immediately, so the server renders the
//! initial state, and wires its rxRust pipeline inside `Effect::new`, which
//! runs only in the browser. That matters: Leptos's tokio integration has no
//! `LocalSet`, so `spawn_local` panics on the server, and both rxRust's
//! `LocalScheduler` timers and `from_signal` change delivery use it.
//!
//! The `wire_*` functions are public so tests (and custom effects) can run
//! the pipelines directly.

use std::{convert::Infallible, time::Duration};

use leptos::prelude::*;
use rx_leptos::prelude::*;
use rxrust::{context::Context, prelude::*};

/// A boxed, single-threaded observable of `T` that never errors.
pub type Stream<T> = Local<<Local<()> as Context>::BoxedCoreObservable<'static, T, Infallible>>;

/// Signals of a debounced, cancelling search box.
pub struct Typeahead {
  pub query: RwSignal<String>,
  pub results: ReadSignal<Vec<String>>,
  pub pending: ReadSignal<bool>,
  pub searches: ReadSignal<u32>,
  pub delivered: ReadSignal<u32>,
}

/// Write side of a [`Typeahead`].
#[derive(Clone, Copy)]
pub struct TypeaheadSinks {
  pub results: WriteSignal<Vec<String>>,
  pub pending: WriteSignal<bool>,
  pub searches: WriteSignal<u32>,
  pub delivered: WriteSignal<u32>,
}

/// Create the signals now and wire the pipeline in the browser.
pub fn typeahead(
  debounce: Duration, backend: impl FnMut(String) -> Stream<Vec<String>> + Clone + 'static,
) -> Typeahead {
  let query = RwSignal::new(String::new());
  let (results, set_results) = signal(Vec::new());
  let (pending, set_pending) = signal(false);
  let (searches, set_searches) = signal(0u32);
  let (delivered, set_delivered) = signal(0u32);
  let sinks = TypeaheadSinks {
    results: set_results,
    pending: set_pending,
    searches: set_searches,
    delivered: set_delivered,
  };

  Effect::new(move |_| wire_typeahead(query, debounce, backend.clone(), sinks));

  Typeahead { query, results, pending, searches, delivered }
}

/// Debounce `query`, drop repeats, run `backend` per settled query and cancel
/// the previous search when a new one starts. Lives as long as the current
/// reactive owner.
pub fn wire_typeahead(
  query: RwSignal<String>, debounce: Duration,
  backend: impl FnMut(String) -> Stream<Vec<String>> + 'static, sinks: TypeaheadSinks,
) {
  query
    .to_observable()
    .debounce(debounce)
    .distinct_until_changed()
    .tap(move |_| {
      sinks.pending.set(true);
      sinks.searches.update(|n| *n += 1);
    })
    .switch_map(backend)
    .tap(move |_| {
      sinks.pending.set(false);
      sinks.delivered.update(|n| *n += 1);
    })
    .feed(sinks.results);
}

/// Signals of a stopwatch.
pub struct Stopwatch {
  pub running: RwSignal<bool>,
  pub reset: RwSignal<u32>,
  pub ticks: ReadSignal<u64>,
}

/// Create the signals now and wire the pipeline in the browser.
pub fn stopwatch(tick: Duration) -> Stopwatch {
  let running = RwSignal::new(false);
  let reset = RwSignal::new(0u32);
  let (ticks, set_ticks) = signal(0u64);

  Effect::new(move |_| wire_stopwatch(running, reset, tick, set_ticks));

  Stopwatch { running, reset, ticks }
}

/// Count `tick` periods while `running`; restart on `reset`.
pub fn wire_stopwatch(
  running: RwSignal<bool>, reset: RwSignal<u32>, tick: Duration, set_ticks: WriteSignal<u64>,
) {
  reset
    .to_observable()
    .switch_map(move |_| {
      running
        .to_observable()
        .switch_map(move |on| {
          if on {
            Local::interval(tick).map(|_| 1u64).box_it()
          } else {
            Local::from_iter(Vec::<u64>::new()).box_it()
          }
        })
        .scan(0u64, |total, n| total + n)
        .start_with(vec![0])
    })
    .feed(set_ticks);
}

/// Feed the throttled pointer position over `target` into `set_position`.
#[cfg(target_arch = "wasm32")]
pub fn wire_mouse(
  target: &web_sys::EventTarget, throttle: Duration, set_position: WriteSignal<(i32, i32)>,
) {
  use wasm_bindgen::JsCast;

  from_event(target, "mousemove")
    .throttle_time(throttle, ThrottleEdge::leading())
    .map(|event: web_sys::Event| {
      let mouse: web_sys::MouseEvent = event.unchecked_into();
      (mouse.client_x(), mouse.client_y())
    })
    .feed(set_position);
}

/// Running statistics of an `animation_frames` stream.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameStats {
  /// Frames seen since the model was created (pauses do not reset it).
  pub frames: u64,
  /// Milliseconds since the current run started.
  pub elapsed_ms: f64,
  /// Frames per second, from the gap to the previous frame.
  pub fps: f64,
}

/// Signals of the frames panel.
pub struct Frames {
  /// Set `false` to stop requesting frames.
  pub running: RwSignal<bool>,
  pub stats: ReadSignal<FrameStats>,
}

/// Create the signals now and wire `animation_frames` in the browser; the
/// server renders zero frames.
pub fn frames() -> Frames {
  let running = RwSignal::new(true);
  let (stats, set_stats) = signal(FrameStats::default());

  Effect::new(move |_| {
    #[cfg(target_arch = "wasm32")]
    wire_frames(running, set_stats);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (running, set_stats);
  });

  Frames { running, stats }
}

/// Count animation frames while `running`; pausing stops the frame requests
/// entirely rather than ignoring them.
#[cfg(target_arch = "wasm32")]
pub fn wire_frames(running: RwSignal<bool>, set_stats: WriteSignal<FrameStats>) {
  running
    .to_observable()
    .switch_map(|on| {
      if on {
        animation_frames().box_it()
      } else {
        Local::from_iter(Vec::<AnimationFrame>::new()).box_it()
      }
    })
    .scan((FrameStats::default(), None::<f64>), |(stats, previous), frame: AnimationFrame| {
      let fps = previous.map_or(0.0, |p| 1000.0 / (frame.timestamp - p).max(1.0));
      (
        FrameStats { frames: stats.frames + 1, elapsed_ms: frame.elapsed, fps },
        Some(frame.timestamp),
      )
    })
    .map(|(stats, _): (FrameStats, Option<f64>)| stats)
    .feed(set_stats);
}

/// Signals of the fetch panel.
pub struct Loader {
  /// Bump to (re)load; a load already in flight is aborted. A counter
  /// rather than a `Subject` so the view captures nothing non-`Send`, which
  /// server rendering requires.
  pub load: RwSignal<u32>,
  /// The strings from the last successful response.
  pub items: ReadSignal<Vec<String>>,
  /// "idle", "loading", "ok" or an error description.
  pub status: ReadSignal<String>,
}

/// Create the signals now and wire `from_fetch` in the browser; the server
/// renders the idle state.
pub fn loader(url: &'static str) -> Loader {
  let load = RwSignal::new(0u32);
  let (items, set_items) = signal(Vec::new());
  let (status, set_status) = signal("idle".to_string());

  Effect::new(move |_| {
    #[cfg(target_arch = "wasm32")]
    wire_loader(load, url, set_items, set_status);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (load, url, set_items, set_status);
  });

  Loader { load, items, status }
}

/// Fetch `url` (a JSON array of strings) every time `load` changes; a new
/// load cancels the previous request through `switch_map`.
#[cfg(target_arch = "wasm32")]
pub fn wire_loader(
  load: RwSignal<u32>, url: &'static str, set_items: WriteSignal<Vec<String>>,
  set_status: WriteSignal<String>,
) {
  use wasm_bindgen::JsValue;
  use wasm_bindgen_futures::JsFuture;
  use web_sys::Response;

  load
    .to_observable()
    .skip(1) // the current value is not a click
    .tap(move |_| set_status.set("loading".into()))
    .switch_map(move |_| {
      from_fetch(url)
        .map(Ok::<Response, JsValue>)
        .catch_error(|err: JsValue| Local::of(Err(err)))
        .switch_map(|result: Result<Response, JsValue>| match result.and_then(|r| r.json()) {
          Ok(promise) => Local::from_future(JsFuture::from(promise)).box_it(),
          Err(err) => Local::of(Err(err)).box_it(),
        })
    })
    .map(|result: Result<JsValue, JsValue>| {
      result
        .map(|value| {
          js_sys::Array::from(&value)
            .iter()
            .filter_map(|v| v.as_string())
            .collect::<Vec<String>>()
        })
        .map_err(|err| {
          err
            .as_string()
            .unwrap_or_else(|| format!("{err:?}"))
        })
    })
    .tap(move |result: &Result<Vec<String>, String>| match result {
      Ok(_) => set_status.set("ok".into()),
      Err(err) => set_status.set(format!("error: {err}")),
    })
    .map(|result: Result<Vec<String>, String>| result.unwrap_or_default())
    .feed(set_items);
}
