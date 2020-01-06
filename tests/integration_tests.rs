use syncbus::Bus;

#[derive(Copy, Clone, PartialEq, Debug)]
enum Value {
    A,
    B,
}

#[test]
fn example() {
    let mut bus = Bus::<Value>::new(10);
    let mut rx1 = bus.add_rx();
    let mut rx2 = bus.add_rx();

    bus.broadcast(Value::A);
    bus.broadcast(Value::B);

    assert_eq!(rx1.recv(), vec![Value::A, Value::B]);
    assert_eq!(rx2.recv(), vec![Value::A, Value::B]);
}
