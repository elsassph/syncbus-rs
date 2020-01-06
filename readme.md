# Rust "syncbus" crate

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
