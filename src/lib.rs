// Copyright 2017 Andrew D. Straw.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Track changes to data and notify listeners.
//!
//! The main API feature is the [`DataTracker`](./struct.DataTracker.html) struct, which
//! takes ownership of a value.
//!
//! The principle of operation is that
//! [`DataTracker::as_tracked_mut()`](./struct.DataTracker.html#method.as_tracked_mut)
//! returns a mutable [`Modifier`](./struct.Modifier.html).
//! `Modifier` has two key properties:
//!
//! - It implements the `DerefMut` trait and allows ergonomic access to the
//! underlying data.
//! - It implements the `Drop` trait which checks if the underlying
//! data was changed and, if so, notifies the listeners.
//!
//! Futhermore, [`DataTracker::as_ref()`](./struct.DataTracker.html#method.as_ref)
//! returns a (non-mutable) reference to the data for cases when only
//! read-only access to the data is needed.
//!
//! To implement tracking, when [`Modifier`](./struct.Modifier.html) is created,
//! a copy of the original data is made and when `Modifier` is dropped, an equality
//! check is performed. If the original and the new data are not equal, the callbacks
//! are called with references to the old and the new values.
//!
//! ```
//! use data_tracker::DataTracker;
//!
//! // Create a new type to be tracked. Must implement Clone and PartialEq.
//! #[derive(Debug, Clone, PartialEq)]
//! struct MyData {
//!     a: u8,
//! }
//!
//! // Create some data.
//! let data = MyData {a: 1};
//!
//! // Transfer ownership of data to the tracker.
//! let mut tracked_data = DataTracker::new(data);
//!
//! // Register a listener which is called when a data change is noticed.
//! let key = 0; // Keep the key to remove the callback later.
//!
//! // At the time or writing, the rust compiler could not infer the type
//! // of the closure and therefore I needed to annotate the argument types.
//! tracked_data.add_listener(key, Box::new(|old_value: &MyData, new_value: &MyData| {
//!     println!("changed {:?} -> {:?}", old_value, new_value);
//! }));
//!
//! {
//!     // Create x, a (non-mutable) reference to original data.
//!     let x = tracked_data.as_ref();
//!     println!("x.a: {}",x.a);
//! }
//!
//! {
//!     // Create x, which allows modifying the original data and checks for changes when
//!     // it goes out of scope.
//!     let mut x = tracked_data.as_tracked_mut();
//!     x.a = 10;
//!     println!("x.a: {}",x.a);
//!     // When we leave this scope, changes are detected and sent to the listeners.
//! }
//!
//! // Remove our callback.
//! tracked_data.remove_listener(&key);
//! ```

use std::collections::HashMap;
use std::hash::Hash;
use std::cmp::Eq;

/// Trait defining change notification callback function.
#[cfg(not(feature = "no_send"))]
pub trait OnChanged<T>: Send {
    fn on_changed(&self, old_value: &T, new_value: &T) -> ();
}

#[cfg(feature = "no_send")]
pub trait OnChanged<T> {
    fn on_changed(&self, old_value: &T, new_value: &T) -> ();
}

#[cfg(not(feature = "no_send"))]
impl<F, T> OnChanged<T> for F
    where F: Fn(&T, &T) -> () + Send
{
    fn on_changed(&self, old_value: &T, new_value: &T) -> () {
        self(old_value, new_value)
    }
}

#[cfg(feature = "no_send")]
impl<F, T> OnChanged<T> for F
    where F: Fn(&T, &T) -> ()
{
    fn on_changed(&self, old_value: &T, new_value: &T) -> () {
        self(old_value, new_value)
    }
}

struct Inner<T, K>
    where T: Clone + PartialEq,
          K: Hash + Eq
{
    value: T,
    fn_map: HashMap<K, Box<OnChanged<T>>>,
}

impl<T, K> Inner<T, K>
    where T: Clone + PartialEq,
          K: Hash + Eq
{
    fn add_listener(&mut self, key: K, f: Box<OnChanged<T>>) -> Option<Box<OnChanged<T>>> {
        self.fn_map.insert(key, f)
    }
    fn remove_listener(&mut self, key: &K) -> Option<Box<OnChanged<T>>> {
        self.fn_map.remove(key)
    }
    fn notify_listeners(&self, modifier: &Modifier<T, K>) {
        let orig_value = &modifier.orig_copy;
        let new_value: &T = modifier;
        for on_changed_obj in self.fn_map.values() {
            on_changed_obj.on_changed(orig_value, new_value);
        }
    }
}

/// Allow viewing and modifying data owned by `DataTracker`.
///
/// Create an instance of this by calling
/// [`DataTracker::as_tracked_mut()`](./struct.DataTracker.html#method.as_tracked_mut).
pub struct Modifier<'a, T, K>
    where T: 'a + Clone + PartialEq,
          K: 'a + Hash + Eq
{
    orig_copy: T,
    inner_ref: &'a mut Inner<T, K>,
}

impl<'a, T, K> Modifier<'a, T, K>
    where T: 'a + Clone + PartialEq,
          K: 'a + Hash + Eq
{
    fn new(inner: &'a mut Inner<T, K>) -> Modifier<'a, T, K> {
        let orig_copy: T = inner.value.clone();
        Modifier {
            orig_copy: orig_copy,
            inner_ref: inner,
        }
    }
}

impl<'a, T, K> std::ops::Deref for Modifier<'a, T, K>
    where T: 'a + Clone + PartialEq,
          K: 'a + Hash + Eq
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner_ref.value
    }
}

impl<'a, T, K> std::ops::DerefMut for Modifier<'a, T, K>
    where T: 'a + Clone + PartialEq,
          K: 'a + Hash + Eq
{
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner_ref.value
    }
}

impl<'a, T, K> Drop for Modifier<'a, T, K>
    where T: 'a + Clone + PartialEq,
          K: 'a + Hash + Eq
{
    fn drop(&mut self) {
        if self.orig_copy != self.inner_ref.value {
            self.inner_ref.notify_listeners(self);
        }
    }
}

/// Tracks changes to data and notifies listeners.
///
/// The data to be tracked is type `T`.
///
/// Callbacks are stored in a `HashMap` with keys of type `K`.
///
/// See the [module-level documentation](./) for more details.
pub struct DataTracker<T, K>
    where T: Clone + PartialEq,
          K: Hash + Eq
{
    inner: Inner<T, K>,
}

impl<T, K> DataTracker<T, K>
    where T: Clone + PartialEq,
          K: Hash + Eq
{
    /// Create a new `DataTracker` which takes ownership
    /// of the data of type `T`.
    ///
    /// Callbacks are registered via a key of type `K`.
    pub fn new(value: T) -> DataTracker<T, K> {
        DataTracker {
            inner: Inner {
                value: value,
                fn_map: HashMap::new(),
            },
        }
    }

    /// Add a callback that will be called just after a data change is detected.
    ///
    /// If a previous callback exists with the `key`, the original callback is
    /// returned as `Some(original_callback)`. Otherwise, `None` is returned.
    pub fn add_listener(&mut self,
                        key: K,
                        callback: Box<OnChanged<T>>)
                        -> Option<Box<OnChanged<T>>> {
        self.inner.add_listener(key, callback)
    }

    /// Remove callback.
    ///
    /// If a callback exists with the `key`, it is removed and returned as
    /// `Some(callback)`. Otherwise, `None` is returned.
    pub fn remove_listener(&mut self, key: &K) -> Option<Box<OnChanged<T>>> {
        self.inner.remove_listener(key)
    }

    /// Return a `Modifier` which can be used to modify the owned data.
    pub fn as_tracked_mut(&mut self) -> Modifier<T, K> {
        Modifier::new(&mut self.inner)
    }
}

impl<T, K> AsRef<T> for DataTracker<T, K>
    where T: Clone + PartialEq,
          K: Hash + Eq
{
    fn as_ref(&self) -> &T {
        &self.inner.value
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use super::DataTracker;

    #[test]
    fn track_struct() {
        #[derive(Clone, PartialEq)]
        struct MyData {
            a: u8,
        }

        let change_count = Arc::new(Mutex::new(0));
        let mut tracked_data = DataTracker::new(MyData { a: 1 });

        let cc2 = change_count.clone();
        let key = 0;
        tracked_data.add_listener(key,
                                  Box::new(move |_: &MyData, _: &MyData| {
                                      let ref mut data = *cc2.lock().unwrap();
                                      *data = *data + 1;
                                  }));

        assert!(*change_count.lock().unwrap() == 0);

        {
            let _x = tracked_data.as_ref();
        }
        assert!(*change_count.lock().unwrap() == 0);

        {
            let mut x = tracked_data.as_tracked_mut();
            x.a = 10;
        }
        assert!(*change_count.lock().unwrap() == 1);

        {
            let mut x = tracked_data.as_tracked_mut();
            x.a += 10;
        }
        assert!(*change_count.lock().unwrap() == 2);

        assert!(tracked_data.as_ref().a == 20);
        assert!(*change_count.lock().unwrap() == 2);

        tracked_data.remove_listener(&key);
    }

    #[test]
    fn track_enum() {

        #[derive(Clone, PartialEq)]
        enum MyEnum {
            FirstValue,
            SecondValue,
        }

        let change_count = Arc::new(Mutex::new(0));
        let mut tracked_data = DataTracker::new(MyEnum::FirstValue);

        let cc2 = change_count.clone();
        let key = 0;
        tracked_data.add_listener(key,
                                  Box::new(move |_: &MyEnum, _: &MyEnum| {
                                      let ref mut data = *cc2.lock().unwrap();
                                      *data = *data + 1;
                                  }));

        assert!(*change_count.lock().unwrap() == 0);

        {
            let _x = tracked_data.as_ref();
        }
        assert!(*change_count.lock().unwrap() == 0);

        {
            let mut x = tracked_data.as_tracked_mut();
            *x = MyEnum::SecondValue;
        }
        assert!(*change_count.lock().unwrap() == 1);

        assert!(tracked_data.as_ref() == &MyEnum::SecondValue);
        assert!(*change_count.lock().unwrap() == 1);

        tracked_data.remove_listener(&key);
    }

    #[test]
    fn callback_arg_order() {

        #[derive(Clone, PartialEq)]
        enum MyEnum {
            FirstValue,
            SecondValue,
        }

        let mut tracked_data = DataTracker::new(MyEnum::FirstValue);

        let did_run = Arc::new(Mutex::new(false));
        let did_run_clone = did_run.clone();

        tracked_data.add_listener(0,
                                  Box::new(move |old_value: &MyEnum, new_value: &MyEnum| {
                                      assert!(old_value == &MyEnum::FirstValue);
                                      assert!(new_value == &MyEnum::SecondValue);
                                      let ref mut data = *did_run_clone.lock().unwrap();
                                      *data = true;
                                  }));
        {
            let mut x = tracked_data.as_tracked_mut();
            *x = MyEnum::SecondValue;
        }

        assert!(*did_run.lock().unwrap() == true);
    }

    // Test that instances of DataTracker implement Send, at least if
    // the owned data type T implements Send.
    #[cfg(not(feature = "no_send"))]
    #[test]
    fn track_send_impl() {

        #[derive(Clone, PartialEq)]
        struct MyStruct {
            a: i32,
        }

        let mut tracked_data = DataTracker::new(MyStruct { a: 42 });
        tracked_data.add_listener(0,
                                  Box::new(move |_old_value: &MyStruct, _new_value: &MyStruct| {
                                  }));

        ::std::thread::spawn(move || {
            let mut x = tracked_data.as_tracked_mut();
            (*x).a = 123;
        });
    }

}
