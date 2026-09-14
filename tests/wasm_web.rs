//! `animation_frames` and `from_fetch`, run under `wasm-pack test --node`.
//! Node has a global `fetch`; `requestAnimationFrame` is polyfilled here on
//! top of `setTimeout`.
#![cfg(target_arch = "wasm32")]

use std::{cell::RefCell, rc::Rc};

use rx_leptos::prelude::*;
use rxrust::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;

fn install_raf_polyfill() {
  let _ = js_sys::eval(
    "if (!globalThis.requestAnimationFrame) { globalThis.requestAnimationFrame = cb => \
     setTimeout(() => cb(performance.now()), 5); globalThis.cancelAnimationFrame = id => \
     clearTimeout(id); }",
  );
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
async fn animation_frames_emit_until_unsubscribed() {
  install_raf_polyfill();
  let frames = Rc::new(RefCell::new(Vec::new()));
  let sink = frames.clone();

  let sub =
    animation_frames().subscribe(move |frame: AnimationFrame| sink.borrow_mut().push(frame));
  sleep(60).await;
  let count = frames.borrow().len();
  assert!(count >= 3, "expected several frames, got {count}");
  assert!(
    frames
      .borrow()
      .windows(2)
      .all(|w| w[1].elapsed >= w[0].elapsed),
    "elapsed grows"
  );

  sub.unsubscribe();
  sleep(30).await;
  assert_eq!(frames.borrow().len(), count, "no frames after unsubscribe");
}

#[wasm_bindgen_test]
async fn animation_frames_stop_when_the_observer_closes() {
  install_raf_polyfill();
  let frames = Rc::new(RefCell::new(0));
  let sink = frames.clone();

  animation_frames()
    .take(2)
    .subscribe(move |_| *sink.borrow_mut() += 1);
  sleep(60).await;
  assert_eq!(*frames.borrow(), 2);
}

#[wasm_bindgen_test]
async fn from_fetch_delivers_the_response_and_completes() {
  let result = from_fetch("data:text/plain,hello")
    .into_future()
    .await;
  let response = result.expect("fetch resolved").expect("no error");
  let text = JsFuture::from(response.text().unwrap())
    .await
    .unwrap();
  assert_eq!(text.as_string().as_deref(), Some("hello"));
}

#[wasm_bindgen_test]
async fn from_fetch_unsubscribe_aborts_and_delivers_nothing() {
  let delivered = Rc::new(RefCell::new(false));
  let sink = delivered.clone();
  let sub = from_fetch("data:text/plain,late")
    .on_error(|_| {})
    .subscribe(move |_| *sink.borrow_mut() = true);
  assert!(!sub.is_closed());
  sub.unsubscribe();
  sleep(20).await;
  assert!(!*delivered.borrow(), "aborted request delivers nothing");
}

#[wasm_bindgen_test]
async fn from_fetch_reports_failures_as_errors() {
  let result = from_fetch("http://127.0.0.1:1/unreachable")
    .into_future()
    .await;
  assert!(matches!(result, Ok(Err(_))), "network failure surfaces as the error branch");
}
