use crate::{Closure, Listener};
use accessory::Accessors;
use derivative::Derivative;
use fancy_constructor::new;
use std::marker::PhantomData;
use tokio::sync::mpsc;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use web_sys::Event;

/// Options builder for an event listener.
#[derive(Accessors, Derivative, new)]
#[access(defaults(set(owned, prefix(with), const_fn)))]
#[derivative(
    Clone(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
#[new(const_fn, bounds(T: FromWasmAbi + 'static))]
#[must_use]
pub struct ListenerBuilder<T = Event> {
    /// Set the [`passive`](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener#passive)
    /// option.
    #[access(set)]
    #[new(val(false))]
    passive: bool,

    /// Set the [`capture`](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener#capture)
    /// option.
    #[access(set)]
    #[new(val(false))]
    capture: bool,

    /// Set the [`once`](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener#once)
    /// option.
    #[access(set)]
    #[new(val(false))]
    once: bool,

    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    #[new(val(PhantomData))]
    _arg: PhantomData<T>,
}

impl<T> ListenerBuilder<T>
where
    T: FromWasmAbi + 'static,
{
    /// Build the options into a listener that can be attached to things.
    #[allow(clippy::missing_errors_doc)]
    pub fn build(self) -> Result<Listener<T>, JsValue> {
        let opts = self.build_opts()?;
        let (tx, rx) = mpsc::unbounded_channel();
        let closure = Closure::new(move |event| {
            let _ = tx.send(event);
        });

        Ok(Listener::new(closure, opts, rx))
    }

    fn build_opts(&self) -> Result<Option<js_sys::Object>, JsValue> {
        macro_rules! build {
            ($($opt: ident),+ $(,)?) => {
                if $(!self.$opt)&&+ {
                    Ok(None)
                } else {
                    let obj = js_sys::Object::new();
                    $(if self.$opt {
                        js_sys::Reflect::set(&obj, &stringify!($opt).into(), &true.into())?;
                    })+
                    Ok(Some(obj.unchecked_into()))
                }
            };
        }

        build!(passive, capture, once)
    }
}

impl<T> Listener<T> {
    /// Alias for [`ListenerBuilder::new`].
    pub const fn builder() -> ListenerBuilder<T>
    where
        T: FromWasmAbi + 'static,
    {
        ListenerBuilder::new()
    }
}

impl<T> Default for ListenerBuilder<T>
where
    T: FromWasmAbi + 'static,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
