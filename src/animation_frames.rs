//! `requestAnimationFrame` as an observable (wasm only).

use std::{cell::RefCell, convert::Infallible, rc::Rc};

use rxrust::{
  context::Context,
  observable::{CoreObservable, ObservableType},
  observer::Observer,
  prelude::Local,
  subscription::Subscription,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// One animation frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationFrame {
  /// The frame's `DOMHighResTimeStamp`, as passed to the callback.
  pub timestamp: f64,
  /// Milliseconds since the subscription started.
  pub elapsed: f64,
}

/// An observable that emits once per animation frame until unsubscribed.
///
/// `requestAnimationFrame` is looked up on the global object, so this works
/// in windows, workers and anywhere else the function is defined.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnimationFrames;

impl ObservableType for AnimationFrames {
  type Item<'a>
    = AnimationFrame
  where
    Self: 'a;
  type Err = Infallible;
}

fn global_fn(name: &str) -> Result<js_sys::Function, JsValue> {
  js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str(name))?.dyn_into::<js_sys::Function>()
}

fn now() -> f64 {
  js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("performance"))
    .ok()
    .and_then(|p| js_sys::Reflect::get(&p, &JsValue::from_str("now")).ok())
    .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
    .and_then(|f| f.call0(&js_sys::global()).ok())
    .and_then(|v| v.as_f64())
    .unwrap_or_else(js_sys::Date::now)
}

/// The handle is whatever `requestAnimationFrame` returned (a number in
/// browsers; node polyfills return timer objects), handed back verbatim to
/// `cancelAnimationFrame`.
fn request(callback: &JsValue) -> Option<JsValue> {
  let handle = global_fn("requestAnimationFrame")
    .ok()?
    .call1(&js_sys::global(), callback)
    .ok()?;
  (!handle.is_undefined() && !handle.is_null()).then_some(handle)
}

fn cancel(handle: JsValue) {
  if let Ok(cancel) = global_fn("cancelAnimationFrame") {
    let _ = cancel.call1(&js_sys::global(), &handle);
  }
}

struct FrameState<O> {
  observer: Option<O>,
  callback: Option<Closure<dyn FnMut(f64)>>,
  handle: Option<JsValue>,
  start: f64,
}

impl<O> FrameState<O> {
  /// Stop requesting frames. The callback may be the one running right now,
  /// so it is dropped on a later microtask rather than here.
  fn stop(&mut self) {
    self.observer = None;
    if let Some(handle) = self.handle.take() {
      cancel(handle);
    }
    if let Some(callback) = self.callback.take() {
      wasm_bindgen_futures::spawn_local(async move { drop(callback) });
    }
  }
}

/// Subscription handle: unsubscribing cancels the pending frame request.
pub struct AnimationFrameSubscription<O> {
  state: Rc<RefCell<FrameState<O>>>,
}

impl<O> Subscription for AnimationFrameSubscription<O> {
  fn unsubscribe(self) { self.state.borrow_mut().stop(); }

  fn is_closed(&self) -> bool { self.state.borrow().observer.is_none() }
}

impl<C> CoreObservable<C> for AnimationFrames
where
  C: Context,
  C::Inner: Observer<AnimationFrame, Infallible> + 'static,
{
  type Unsub = AnimationFrameSubscription<C::Inner>;

  fn subscribe(self, context: C) -> Self::Unsub {
    let state = Rc::new(RefCell::new(FrameState {
      observer: Some(context.into_inner()),
      callback: None,
      handle: None,
      start: now(),
    }));

    let callback = {
      let state = state.clone();
      Closure::wrap(Box::new(move |timestamp: f64| {
        // Take the observer out so it can unsubscribe from inside `next`.
        let (observer, start) = {
          let mut st = state.borrow_mut();
          st.handle = None;
          (st.observer.take(), st.start)
        };
        let Some(mut observer) = observer else { return };
        observer.next(AnimationFrame { timestamp, elapsed: timestamp - start });

        let mut st = state.borrow_mut();
        if st.observer.is_some() {
          // Unsubscribed and resubscribed is impossible; this is a stale run.
          return;
        }
        if observer.is_closed() || st.callback.is_none() {
          drop(st);
          state.borrow_mut().stop();
          return;
        }
        st.observer = Some(observer);
        let callback = st
          .callback
          .as_ref()
          .expect("callback present while active");
        st.handle = request(callback.as_ref().unchecked_ref());
        if st.handle.is_none() {
          st.stop();
        }
      }) as Box<dyn FnMut(f64)>)
    };

    {
      let mut st = state.borrow_mut();
      st.handle = request(callback.as_ref().unchecked_ref());
      st.callback = Some(callback);
      if st.handle.is_none() {
        // No requestAnimationFrame here: nothing will ever be emitted.
        st.stop();
      }
    }

    AnimationFrameSubscription { state }
  }
}

/// Emit once per animation frame until unsubscribed.
pub fn animation_frames() -> Local<AnimationFrames> { Local::<()>::lift(AnimationFrames) }
