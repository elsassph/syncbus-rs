/*!
`syncbus` provides a single-threaded, WASM-compatible, single-producer, multi-consumer, polling bus.

## API

API is loosely inspired by the `bus` crate:

The `Bus<T: Copy>` struct is the single producer - pass it around to send simple messages.

Use `bus.add_rx()` to create a new `BusReader`:

- each reader will receive a copy of the messages,
- readers should poll the queue as part of an update loop.

## Usage:

```rust
use syncbus::Bus;

let mut bus = Bus::<Value>::new(10);
let mut rx = bus.add_rx();

bus.broadcast(Value::A);
bus.broadcast(Value::B);

assert_eq!(rx.recv(), vec![Value::A, Value::B]);
```
*/

use std::cell::RefCell;
use std::rc::Rc;

struct RxSlot<T: Copy> {
    index: usize,
    queue: Vec<T>,
}

// Inner message bus shared by Bus and BusReader
struct BusInner<T: Copy> {
    slots: Vec<RxSlot<T>>,
    count: usize,
}
impl<T: Copy> BusInner<T> {
    fn new(capacity: usize) -> BusInner<T> {
        assert!(capacity > 2, "Capacity should be at least 2");

        BusInner::<T> {
            slots: Vec::<RxSlot<T>>::with_capacity(capacity),
            count: 0,
        }
    }

    fn add_rx(&mut self) -> usize {
        let index = self.count;
        self.count += 1;
        self.slots.push(RxSlot::<T> {
            index,
            queue: vec![],
        });
        index
    }

    fn broadcast(&mut self, value: T) {
        for rx in self.slots.iter_mut() {
            rx.queue.push(value);
        }
    }

    fn recv(&mut self, index: usize) -> Vec<T> {
        for rx in self.slots.iter_mut() {
            if rx.index == index {
                return rx.queue.drain(..).collect();
            }
        }
        vec![]
    }

    fn leave(&mut self, index: usize) {
        self.slots.retain(|rx| rx.index != index);
    }
}

/// `BusReader` is the messages consumer.
/// Use `recv()` to poll for messages.
pub struct BusReader<T: Copy> {
    inner: Rc<RefCell<BusInner<T>>>,
    index: usize,
}
impl<T: Copy> Drop for BusReader<T> {
    fn drop(&mut self) {
        self.inner.borrow_mut().leave(self.index);
    }
}
impl<T: Copy> BusReader<T> {
    /// Receive the pending messages (if any) and empty the queue
    /// ```
    /// for msg in reader.recv() {
    ///     match msg {...}
    /// }
    /// ```
    pub fn recv(&mut self) -> Vec<T> {
        self.inner.borrow_mut().recv(self.index)
    }
}

/// `Bus` is the single producer.
/// Use `add_rx()` to create a consumer.
/// Use `broadcast(value)` to push a message in each consumer queue.
pub struct Bus<T: Copy> {
    inner: Rc<RefCell<BusInner<T>>>,
}
impl<T: Copy> Bus<T> {
    /// Create a new `Bus`, with `capacity` to be 2 or more
    pub fn new(capacity: usize) -> Bus<T> {
        let inner = Rc::new(RefCell::new(BusInner::new(capacity)));
        Bus::<T> { inner }
    }

    /// Create a new `BusReader`; it will receive copies of the messages until dropped.
    pub fn add_rx(&mut self) -> BusReader<T> {
        BusReader::<T> {
            inner: Rc::clone(&self.inner),
            index: self.inner.borrow_mut().add_rx(),
        }
    }

    /// Push copies of the value in the reader queues.
    pub fn broadcast(&self, value: T) {
        self.inner.borrow_mut().broadcast(value);
    }
}

//---------- TESTS ------------

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Debug)]
    enum Value {
        A,
        B,
    }

    #[test]
    #[should_panic]
    fn should_enforce_capacity() {
        let _ = Bus::<Value>::new(1);
    }

    #[test]
    fn should_not_crash_broadcasting_without_readers() {
        let bus = Bus::<Value>::new(10);
        bus.broadcast(Value::A);
    }

    #[test]
    fn reader_should_have_incremental_head() {
        let mut bus = Bus::<Value>::new(5);

        {
            let inner = bus.inner.borrow_mut();
            assert_eq!(inner.slots.capacity(), 5);
            assert_eq!(inner.slots.len(), 0);
            assert_eq!(inner.count, 0);
        }

        let mut rxs: Vec<BusReader<Value>> = vec![];
        for i in 0..10 {
            let rx = bus.add_rx();
            assert_eq!(rx.index, i);
            rxs.push(rx);
        }

        assert_eq!(Rc::strong_count(&bus.inner), 11);
        let inner = bus.inner.borrow_mut();
        assert_eq!(inner.slots.capacity(), 10);
        assert_eq!(inner.slots.len(), 10);
        assert_eq!(inner.count, 10);
    }

    #[test]
    fn reader_should_drop_and_release_count() {
        let mut bus = Bus::<Value>::new(5);

        for i in 0..10 {
            let rx = bus.add_rx();
            assert_eq!(rx.index, i);
        }

        assert_eq!(Rc::strong_count(&bus.inner), 1);
        let inner = bus.inner.borrow_mut();
        assert_eq!(inner.slots.capacity(), 5);
        assert_eq!(inner.slots.len(), 0);
        assert_eq!(inner.count, 10);
    }

    #[test]
    fn recv_without_broadcast_should_be_empty() {
        let mut bus = Bus::<Value>::new(5);
        let mut rx1 = bus.add_rx();
        let mut rx2 = bus.add_rx();

        assert_eq!(rx1.recv(), vec![]);
        assert_eq!(rx2.recv(), vec![]);
    }

    #[test]
    fn recv_should_empty_queue_and_return_values() {
        let mut bus = Bus::<Value>::new(5);
        let mut rx1 = bus.add_rx();
        let mut rx2 = bus.add_rx();

        bus.broadcast(Value::A);
        bus.broadcast(Value::B);

        assert_eq!(rx1.recv(), vec![Value::A, Value::B]);
        assert_eq!(rx2.recv(), vec![Value::A, Value::B]);

        assert_eq!(rx1.recv(), vec![]);
        assert_eq!(rx2.recv(), vec![]);
    }

    #[test]
    fn recv_works_when_bus_dropped() {
        let mut bus = Bus::<Value>::new(5);
        let mut rx = bus.add_rx();

        bus.broadcast(Value::A);

        drop(bus);

        assert_eq!(Rc::strong_count(&rx.inner), 1);
        assert_eq!(rx.recv(), vec![Value::A]);

        let weak = Rc::downgrade(&Rc::clone(&rx.inner));

        drop(rx);

        assert!(weak.upgrade().is_none());
    }
}
