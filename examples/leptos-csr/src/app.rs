//! Components. All reactive wiring comes from [`crate::model`].

use std::time::Duration;

use leptos::prelude::*;
use rxrust::prelude::*;

use crate::model::{Stream, stopwatch, typeahead};

const CRATES: [&str; 8] =
  ["rxrust", "rx-leptos", "reactive_graph", "leptos", "rust", "regex", "rayon", "ring"];

/// A slow, fake search backend: prefix matches after 600 ms.
fn search_crates(query: String) -> Stream<Vec<String>> {
  let hits: Vec<String> = CRATES
    .iter()
    .filter(|name| name.starts_with(query.as_str()))
    .map(|name| name.to_string())
    .collect();
  if query.is_empty() {
    Local::of(hits).box_it()
  } else {
    Local::of(hits)
      .delay(Duration::from_millis(600))
      .box_it()
  }
}

#[component]
pub fn App() -> impl IntoView {
  view! {
    <h1>"rx-leptos"</h1>
    <p class="muted">
      "rxRust pipelines driving Leptos signals. Source: "
      <a href="https://github.com/Dev916/rx-leptos/tree/master/examples/leptos-csr">"examples/leptos-csr"</a>
    </p>
    <TypeaheadPanel/>
    <StopwatchPanel/>
    <MousePanel/>
    <FramesPanel/>
    <FetchPanel/>
  }
}

#[component]
fn TypeaheadPanel() -> impl IntoView {
  let model = typeahead(Duration::from_millis(300), search_crates);
  let query = model.query;
  let results = model.results;
  let pending = model.pending;
  let searches = model.searches;
  let delivered = model.delivered;

  view! {
    <section>
      <h2>"Typeahead: " <code>"debounce → distinct_until_changed → switch_map"</code></h2>
      <input
        type="search"
        placeholder="Search crates (try r, re, rx)"
        prop:value=move || query.get()
        on:input=move |ev| query.set(event_target_value(&ev))
      />
      <p class="muted">
        {move || if pending.get() { "Searching…".to_string() } else {
          format!("{} searches started, {} delivered, {} cancelled",
            searches.get(), delivered.get(), searches.get() - delivered.get())
        }}
      </p>
      <ul>
        {move || results.get().into_iter().map(|name| view! { <li>{name}</li> }).collect_view()}
      </ul>
    </section>
  }
}

#[component]
fn StopwatchPanel() -> impl IntoView {
  let model = stopwatch(Duration::from_millis(100));
  let running = model.running;
  let reset = model.reset;
  let ticks = model.ticks;

  view! {
    <section>
      <h2>"Stopwatch: " <code>"switch_map(interval) → scan"</code></h2>
      <p style="font-size: 2rem; margin: 0.25rem 0;">
        {move || format!("{:.1} s", ticks.get() as f64 / 10.0)}
      </p>
      <button on:click=move |_| running.update(|on| *on = !*on)>
        {move || if running.get() { "Pause" } else { "Start" }}
      </button>
      <button on:click=move |_| reset.update(|n| *n += 1)>"Reset"</button>
    </section>
  }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn MousePanel() -> impl IntoView {
  use crate::model::mouse_position;

  let position = mouse_position(&document(), Duration::from_millis(50));

  view! {
    <section>
      <h2>"Mouse: " <code>"from_event → throttle_time"</code></h2>
      <p>{move || { let (x, y) = position.get(); format!("x = {x}, y = {y}") }}</p>
    </section>
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn MousePanel() -> impl IntoView {
  view! {
    <section>
      <h2>"Mouse: " <code>"from_event → throttle_time"</code></h2>
      <p class="muted">"Only available in the browser build."</p>
    </section>
  }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn FramesPanel() -> impl IntoView {
  use crate::model::frame_stats;

  let running = RwSignal::new(true);
  let stats = frame_stats(running);

  view! {
    <section>
      <h2>"Frames: " <code>"animation_frames → scan"</code></h2>
      <p>{move || {
        let s = stats.get();
        format!("{} frames, {:.0} ms into this run, {:.0} fps", s.frames, s.elapsed_ms, s.fps)
      }}</p>
      <div style="height: 8px; background: #eee; border-radius: 4px;">
        <div style=move || format!(
          "height: 8px; width: {:.1}%; background: #58a; border-radius: 4px;",
          (stats.get().elapsed_ms / 2000.0 % 1.0) * 100.0
        )></div>
      </div>
      <p>
        <button on:click=move |_| running.update(|on| *on = !*on)>
          {move || if running.get() { "Pause" } else { "Resume" }}
        </button>
      </p>
    </section>
  }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn FetchPanel() -> impl IntoView {
  use crate::model::json_loader;

  let loader = json_loader("/data.json");
  let mut load = loader.load.clone();
  let (items, status) = (loader.items, loader.status);

  view! {
    <section>
      <h2>"Fetch: " <code>"from_fetch → switch_map"</code></h2>
      <button on:click=move |_| load.next(())>"Load data.json"</button>
      <p class="muted">{move || status.get()}</p>
      <ul>
        {move || items.get().into_iter().map(|name| view! { <li>{name}</li> }).collect_view()}
      </ul>
    </section>
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn FramesPanel() -> impl IntoView {
  view! {
    <section>
      <h2>"Frames: " <code>"animation_frames → scan"</code></h2>
      <p class="muted">"Only available in the browser build."</p>
    </section>
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn FetchPanel() -> impl IntoView {
  view! {
    <section>
      <h2>"Fetch: " <code>"from_fetch → switch_map"</code></h2>
      <p class="muted">"Only available in the browser build."</p>
    </section>
  }
}
