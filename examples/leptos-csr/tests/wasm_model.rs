//! The browser-only models, run under `wasm-pack test --node`.
#![cfg(target_arch = "wasm32")]

use leptos::prelude::*;
use leptos_csr_example::model::{frame_stats, json_loader};
use rxrust::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;

fn setup() -> Owner {
  let _ = any_spawner::Executor::init_wasm_bindgen();
  let _ = js_sys::eval(
    "if (!globalThis.requestAnimationFrame) { globalThis.requestAnimationFrame = cb => \
     setTimeout(() => cb(performance.now()), 5); globalThis.cancelAnimationFrame = id => \
     clearTimeout(id); }",
  );
  Owner::new()
}

async fn sleep(ms: i32) {
  let promise = js_sys::Promise::new(&mut |resolve, _| {
    let _ = js_sys::Reflect::get(&js_sys::global(), &"setTimeout".into())
      .unwrap()
      .dyn_into::<js_sys::Function>()
      .unwrap()
      .call2(&js_sys::global(), &resolve, &ms.into());
  });
  let _ = JsFuture::from(promise).await;
}

#[wasm_bindgen_test]
async fn frame_stats_count_while_running_and_pause() {
  let owner = setup();
  let running = RwSignal::new(true);
  let stats = owner.with(|| frame_stats(running));

  sleep(60).await;
  let seen = stats.get_untracked();
  assert!(seen.frames >= 3, "expected frames, got {}", seen.frames);
  assert!(seen.fps > 0.0);

  running.set(false);
  sleep(30).await;
  let paused = stats.get_untracked().frames;
  sleep(40).await;
  assert_eq!(stats.get_untracked().frames, paused, "no frames while paused");

  running.set(true);
  sleep(40).await;
  assert!(stats.get_untracked().frames > paused, "counting resumes");
  owner.cleanup();
}

#[wasm_bindgen_test]
async fn json_loader_loads_on_demand_and_reports_status() {
  let owner = setup();
  let mut loader = owner.with(|| json_loader("data:application/json,[\"a\",\"b\"]"));
  assert_eq!(loader.status.get_untracked(), "idle");
  assert!(loader.items.get_untracked().is_empty());

  loader.load.next(());
  sleep(20).await;
  assert_eq!(loader.items.get_untracked(), vec!["a", "b"]);
  assert_eq!(loader.status.get_untracked(), "ok");
  owner.cleanup();
}

#[wasm_bindgen_test]
async fn json_loader_reports_failures() {
  let owner = setup();
  let mut loader = owner.with(|| json_loader("http://127.0.0.1:1/nope"));
  loader.load.next(());
  sleep(200).await;
  assert!(
    loader
      .status
      .get_untracked()
      .starts_with("error:"),
    "{}",
    loader.status.get_untracked()
  );
  assert!(loader.items.get_untracked().is_empty());
  owner.cleanup();
}
