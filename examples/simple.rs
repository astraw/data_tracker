extern crate data_tracker;

use data_tracker::DataTracker;

#[derive(Debug, Clone, PartialEq)]
struct MyData {
    a: u8,
}

fn main() {
    let mut data = DataTracker::new(MyData { a: 1 });
    let key = 0; // Keep the key to remove the callback later.
    data.add_listener(key,
                      Box::new(|old_value: &MyData, new_value: &MyData| {
                          println!("changed {:?} -> {:?}", old_value, new_value);
                      }));

    {
        // Create x, a (non-mutable) reference to original data.
        let x = data.as_ref();
        println!("x.a: {}", x.a);
    }

    {
        // Create x, which allows modifying the original data and checks for changes when
        // it goes out of scope.
        let mut x = data.as_tracked_mut();
        x.a = 10;
        println!("x.a: {}", x.a);
        // When we leave this scope, changes are detected and sent to the listeners.
    }

}
