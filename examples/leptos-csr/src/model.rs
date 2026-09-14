//! View-independent reactive models.
//!
//! Each function wires an rxRust pipeline between Leptos signals and returns
//! the signals. The subscriptions belong to the reactive owner that called
//! the function (`to_signal`), so a component that creates a model also
//! disposes it.

use std::{convert::Infallible, time::Duration};

use leptos::prelude::*;
use rx_leptos::prelude::*;
use rxrust::{context::Context, prelude::*};

/// A boxed, single-threaded observable of `T` that never errors. Backends
/// return this so the models stay free of generic bounds.
pub type Stream<T> = Local<<Local<()> as Context>::BoxedCoreObservable<'static, T, Infallible>>;

/// Signals of a debounced, cancelling search box.
pub struct Typeahead {
  /// What the user typed. Write this from the input.
  pub query: RwSignal<String>,
  /// Results of the most recent settled query.
  pub results: ReadSignal<Vec<String>>,
  /// `true` while a search is in flight.
  pub pending: ReadSignal<bool>,
  /// Searches started so far.
  pub searches: ReadSignal<u32>,
  /// Searches whose results arrived (the rest were cancelled).
  pub delivered: ReadSignal<u32>,
}

/// Debounce `query`, drop repeats, run `backend` for each settled query and
/// cancel the previous search when a new one starts.
pub fn typeahead(
  debounce: Duration, backend: impl FnMut(String) -> Stream<Vec<String>> + 'static,
) -> Typeahead {
  let query = RwSignal::new(String::new());
  let (pending, set_pending) = signal(false);
  let (searches, set_searches) = signal(0u32);
  let (delivered, set_delivered) = signal(0u32);

  let results = to_signal(
    from_signal(query)
      .debounce(debounce)
      .distinct_until_changed()
      .tap(move |_| {
        set_pending.set(true);
        set_searches.update(|n| *n += 1);
      })
      .switch_map(backend)
      .tap(move |_| {
        set_pending.set(false);
        set_delivered.update(|n| *n += 1);
      }),
    Vec::new(),
  );

  Typeahead { query, results, pending, searches, delivered }
}

/// Signals of a stopwatch.
pub struct Stopwatch {
  /// Set `true` to run, `false` to pause.
  pub running: RwSignal<bool>,
  /// Bump to restart from zero.
  pub reset: RwSignal<u32>,
  /// Ticks counted since the last reset.
  pub ticks: ReadSignal<u64>,
}

/// Count `tick` periods while `running` is `true`; restart on `reset`.
pub fn stopwatch(tick: Duration) -> Stopwatch {
  let running = RwSignal::new(false);
  let reset = RwSignal::new(0u32);

  let ticks = to_signal(
    from_signal(reset).switch_map(move |_| {
      from_signal(running)
        .switch_map(move |on| {
          if on {
            Local::interval(tick).map(|_| 1u64).box_it()
          } else {
            Local::from_iter(Vec::<u64>::new()).box_it()
          }
        })
        .scan(0u64, |total, n| total + n)
        .start_with(vec![0])
    }),
    0,
  );

  Stopwatch { running, reset, ticks }
}

/// Latest pointer position over `target`, throttled to one update per
/// `throttle`.
#[cfg(target_arch = "wasm32")]
pub fn mouse_position(target: &web_sys::EventTarget, throttle: Duration) -> ReadSignal<(i32, i32)> {
  use wasm_bindgen::JsCast;

  to_signal(
    from_event(target, "mousemove")
      .throttle_time(throttle, ThrottleEdge::leading())
      .map(|event: web_sys::Event| {
        let mouse: web_sys::MouseEvent = event.unchecked_into();
        (mouse.client_x(), mouse.client_y())
      }),
    (0, 0),
  )
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

/// Count animation frames while `running`; pausing stops the frame requests
/// entirely rather than ignoring them.
#[cfg(target_arch = "wasm32")]
pub fn frame_stats(running: RwSignal<bool>) -> ReadSignal<FrameStats> {
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
    .to_signal(FrameStats::default())
}

/// A button-driven JSON loader.
#[cfg(target_arch = "wasm32")]
pub struct Loader {
  /// Emit to (re)load; a load already in flight is aborted.
  pub load: LocalSubject<'static, (), Infallible>,
  /// The strings from the last successful response.
  pub items: ReadSignal<Vec<String>>,
  /// "idle", "loading", "ok" or an error description.
  pub status: ReadSignal<String>,
}

/// Fetch `url` (a JSON array of strings) every time `load` emits; a new
/// load cancels the previous request through `switch_map`.
#[cfg(target_arch = "wasm32")]
pub fn json_loader(url: &'static str) -> Loader {
  use wasm_bindgen::JsValue;
  use wasm_bindgen_futures::JsFuture;
  use web_sys::Response;

  let load = use_subject::<()>();
  let (status, set_status) = signal("idle".to_string());

  let items = load
    .clone()
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
    .to_signal(Vec::new());

  Loader { load, items, status }
}
