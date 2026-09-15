//! The browser-only wiring, run under
//! `wasm-pack test --node -- --no-default-features --features hydrate`.
#![cfg(target_arch = "wasm32")]

use leptos::prelude::*;
use leptos_ssr_example::model::{FrameStats, wire_frames, wire_loader};
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
  Owner::new_root(None)
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
async fn wire_frames_counts_and_pauses() {
  let owner = setup();
  let (running, stats) = owner.with(|| {
    let running = RwSignal::new(true);
    let (stats, set_stats) = signal(FrameStats::default());
    wire_frames(running, set_stats);
    (running, stats)
  });

  sleep(60).await;
  let seen = owner.with(|| stats.get_untracked());
  assert!(seen.frames >= 3, "expected frames, got {}", seen.frames);
  assert!((0.0..1000.0).contains(&seen.elapsed_ms), "elapsed from subscribe: {}", seen.elapsed_ms);

  owner.with(|| running.set(false));
  sleep(30).await;
  let paused = owner.with(|| stats.get_untracked()).frames;
  sleep(40).await;
  assert_eq!(owner.with(|| stats.get_untracked()).frames, paused, "no frames while paused");
  owner.cleanup();
}

#[wasm_bindgen_test]
async fn wire_loader_fetches_on_bump_only() {
  let owner = setup();
  let (load, items, status) = owner.with(|| {
    let load = RwSignal::new(0u32);
    let (items, set_items) = signal(Vec::new());
    let (status, set_status) = signal("idle".to_string());
    wire_loader(load, "data:application/json,[\"a\",\"b\"]", set_items, set_status);
    (load, items, status)
  });

  sleep(20).await;
  assert_eq!(owner.with(|| status.get_untracked()), "idle", "wiring alone does not fetch");

  owner.with(|| load.update(|n| *n += 1));
  sleep(30).await;
  assert_eq!(owner.with(|| items.get_untracked()), vec!["a", "b"]);
  assert_eq!(owner.with(|| status.get_untracked()), "ok");
  owner.cleanup();
}
