//! Resource to observable.

use std::convert::Infallible;

use reactive_graph::traits::Get;
use rxrust::{
  context::Context,
  observable::{CoreObservable, ObservableType},
  observer::Observer,
  prelude::Local,
};

use crate::from_signal::{Node, SignalSubscription};

/// An observable that emits every resolved value of a Leptos `Resource`,
/// `LocalResource`, or any other `Get<Value = Option<T>>` type.
///
/// Subscribing reads the resource once and emits its value if it has already
/// resolved; the pending `None` is skipped. Each later resolution (a refetch
/// after a source signal changed, or `refetch()`) is delivered on the next
/// tick of the app's executor, exactly like [`FromSignal`](crate::FromSignal),
/// which this is built on. A refetch that resolves to an equal value emits
/// again; add `distinct_until_changed` if that matters.
///
/// Leptos keeps the previous value in a resource while a refetch is in
/// flight, so there is no `None` between two resolutions.
///
/// Reading a `Resource` outside `<Suspense>` logs a warning in `hydrate`
/// builds with debug assertions on; `LocalResource` does not. The observable
/// completes only if the resource is disposed; otherwise unsubscribe to
/// detach it.
pub struct FromResource<S> {
  resource: S,
}

impl<S: Clone> Clone for FromResource<S> {
  fn clone(&self) -> Self { Self { resource: self.resource.clone() } }
}

impl<S, T> ObservableType for FromResource<S>
where
  S: Get<Value = Option<T>>,
{
  type Item<'a>
    = T
  where
    Self: 'a;
  type Err = Infallible;
}

/// Observer adapter that forwards `Some` values and drops the pending `None`.
pub struct Resolved<O>(O);

impl<O, T> Observer<Option<T>, Infallible> for Resolved<O>
where
  O: Observer<T, Infallible>,
{
  fn next(&mut self, value: Option<T>) {
    if let Some(value) = value {
      self.0.next(value);
    }
  }

  fn error(self, never: Infallible) { match never {} }

  fn complete(self) { self.0.complete() }

  fn is_closed(&self) -> bool { self.0.is_closed() }
}

impl<S, T, C> CoreObservable<C> for FromResource<S>
where
  C: Context,
  C::Inner: Observer<T, Infallible> + 'static,
  S: Get<Value = Option<T>> + 'static,
{
  type Unsub = SignalSubscription;

  fn subscribe(self, context: C) -> Self::Unsub {
    let node = Node::start(self.resource, Resolved(context.into_inner()));
    SignalSubscription { node }
  }
}

/// Mirror the resolved values of a resource as a `Local` observable. See
/// [`FromResource`].
///
/// # Examples
///
/// ```
/// use std::{cell::RefCell, rc::Rc};
///
/// use any_spawner::Executor;
/// use reactive_graph::{signal::RwSignal, traits::Set};
/// use rx_leptos::from_resource;
/// use rxrust::prelude::*;
///
/// // Leptos does this for you when it mounts the app.
/// let _ = Executor::init_futures_executor();
///
/// // Any `Get<Value = Option<T>>` works; in an app this is a `LocalResource`.
/// let user = RwSignal::new(None::<&str>);
/// let seen = Rc::new(RefCell::new(Vec::new()));
/// let sink = seen.clone();
/// let _sub = from_resource(user).subscribe(move |name| sink.borrow_mut().push(name));
/// assert!(seen.borrow().is_empty(), "still pending");
/// user.set(Some("ada"));
/// Executor::poll_local();
/// assert_eq!(*seen.borrow(), vec!["ada"]);
/// ```
pub fn from_resource<S, T>(resource: S) -> Local<FromResource<S>>
where
  S: Get<Value = Option<T>> + 'static,
{
  Local::<()>::lift(FromResource { resource })
}
