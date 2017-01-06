# data_tracker - tracks changes to data and notifies listeners [![Version][version-img]][version-url] [![Status][status-img]][status-url] [![Doc][doc-img]][doc-url]

## Documentation

Documentation is available [here](https://docs.rs/data_tracker/).

## Overview

The main API feature is the `DataTracker` struct, which
takes ownership of a value.

The principle of operation is that
`DataTracker::as_tracked_mut()` returns a mutable `Modifier`.
`Modifier` has two key properties:

- It implements the `DerefMut` trait and allows ergonomic access to the
underlying data.
- It implements the `Drop` trait which checks if the underlying
data was changed and, if so, notifies the listeners.

Futhermore, `DataTracker::as_ref()` returns a (non-mutable) reference to
the data for cases when only read-only access to the data is needed.

To implement tracking, when `Modifier` is created, a copy of the original data
is made and when `Modifier` is dropped, an equality check is performed. If the
original and the new data are not equal, the callbacks are called with
references to the old and the new values.

## Example

```rust
use data_tracker::DataTracker;

// Create a new type to be tracked. Must implement Clone and PartialEq.
#[derive(Debug, Clone, PartialEq)]
struct MyData {
    a: u8,
}

// Create some data.
let data = MyData {a: 1};

// Transfer ownership of data to the tracker.
let mut tracked_data = DataTracker::new(data);

// Register a listener which is called when a data change is noticed.
let key = 0; // Keep the key to remove the callback later.
tracked_data.add_listener(key, Box::new(|old_value, new_value| {
    println!("changed {:?} -> {:?}", old_value, new_value);
}));

{
    // Create x, a (non-mutable) reference to original data.
    let x = tracked_data.as_ref();
    println!("x.a: {}",x.a);
}

{
    // Create x, which allows modifying the original data and checks for changes when
    // it goes out of scope.
    let mut x = tracked_data.as_tracked_mut();
    x.a = 10;
    println!("x.a: {}",x.a);
    // When we leave this scope, changes are detected and sent to the listeners.
}

// Remove our callback.
tracked_data.remove_listener(&key);
```


## License

Licensed under either of

* Apache License, Version 2.0,
  (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT)
  at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

## Code of conduct

Anyone who interacts with data_tracker in any space including but not limited to
this GitHub repository is expected to follow our
[code of conduct](https://github.com/astraw/data_tracker/blob/master/code_of_conduct.md)

[version-img]: https://img.shields.io/crates/v/data_tracker.svg
[version-url]: https://crates.io/crates/data_tracker
[status-img]: https://travis-ci.org/astraw/data_tracker.svg?branch=master
[status-url]: https://travis-ci.org/astraw/data_tracker
[doc-img]: https://docs.rs/data_tracker/badge.svg
[doc-url]: https://docs.rs/data_tracker/
