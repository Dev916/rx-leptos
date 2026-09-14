//! `fetch` as an observable (wasm only).

use std::{cell::RefCell, rc::Rc};

use rxrust::{
  context::Context,
  observable::{CoreObservable, ObservableType},
  observer::Observer,
  prelude::Local,
  subscription::Subscription,
};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{AbortController, RequestInit, Response};

/// An observable that performs one `fetch` per subscription, emits the
/// `Response` and completes; a failed request errors with the `JsValue`
/// the promise rejected with. Unsubscribing aborts the request.
///
/// `fetch` is looked up on the global object, so this works in windows,
/// workers and node.
#[derive(Clone)]
pub struct FromFetch {
  url: String,
  init: RequestInit,
}

impl ObservableType for FromFetch {
  type Item<'a>
    = Response
  where
    Self: 'a;
  type Err = JsValue;
}

/// Subscription handle: unsubscribing aborts the request and drops the
/// observer, so nothing is delivered afterwards.
pub struct FetchSubscription<O> {
  controller: AbortController,
  observer: Rc<RefCell<Option<O>>>,
}

impl<O> Subscription for FetchSubscription<O> {
  fn unsubscribe(self) {
    let observer = self.observer.borrow_mut().take();
    drop(observer);
    self.controller.abort();
  }

  fn is_closed(&self) -> bool { self.observer.borrow().is_none() }
}

fn call_fetch(url: &str, init: &RequestInit) -> Result<js_sys::Promise, JsValue> {
  let global = js_sys::global();
  let fetch =
    js_sys::Reflect::get(&global, &JsValue::from_str("fetch"))?.dyn_into::<js_sys::Function>()?;
  fetch
    .call2(&global, &JsValue::from_str(url), init)?
    .dyn_into::<js_sys::Promise>()
}

impl<C> CoreObservable<C> for FromFetch
where
  C: Context,
  C::Inner: Observer<Response, JsValue> + 'static,
{
  type Unsub = FetchSubscription<C::Inner>;

  fn subscribe(self, context: C) -> Self::Unsub {
    let observer = Rc::new(RefCell::new(Some(context.into_inner())));
    let controller = AbortController::new().expect("AbortController");
    self.init.set_signal(Some(&controller.signal()));

    let task_observer = observer.clone();
    match call_fetch(&self.url, &self.init) {
      Ok(promise) => {
        wasm_bindgen_futures::spawn_local(async move {
          let result = wasm_bindgen_futures::JsFuture::from(promise).await;
          // Take the observer out first: it may unsubscribe from inside.
          let observer = task_observer.borrow_mut().take();
          let Some(mut observer) = observer else { return };
          match result {
            Ok(response) => {
              observer.next(response.unchecked_into::<Response>());
              observer.complete();
            }
            Err(err) => observer.error(err),
          }
        });
      }
      Err(err) => {
        let observer = observer.borrow_mut().take();
        if let Some(observer) = observer {
          observer.error(err);
        }
      }
    }

    FetchSubscription { controller, observer }
  }
}

/// Fetch `url` with default options.
pub fn from_fetch(url: &str) -> Local<FromFetch> { from_fetch_with(url, RequestInit::new()) }

/// Fetch `url` with `init` (method, headers, body, ...). The request's
/// abort signal is set by the subscription.
pub fn from_fetch_with(url: &str, init: RequestInit) -> Local<FromFetch> {
  Local::<()>::lift(FromFetch { url: url.to_string(), init })
}
