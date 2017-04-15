# epoll-rs
A safe and sufficient wrapper around Linux's EPoll interface.

# Example
```rust
let mut epoll = EPoll::new();

// Register a file-like object onto the epoll.
// The last parameter is a user-defined identifier
epoll.add(&some_pipe, EPOLLIN, 0)?;
epoll.add(&timer, EPOLLIN, 1)?;

let mut events = [Event::default(); 2];
let event_count = epoll.wait(&mut events, Timeout::Milliseconds(500))?;
for e in &events[..event_count] {
    match e.data {
        0 => { /* Do something with the socket */ },
        1 => { /* Do something with the timer  */ },
        _ => unreachable!()
    };
}
```

# TODO:
Some kind of a more sophisticated event-loop.