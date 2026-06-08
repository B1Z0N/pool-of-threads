# pool-of-threads

Learning Rust by building a thread pool. Also figuring out how work-stealing schedulers work.

## Why

I'm refreshing Rust and wanted a project that forces me to actually use:
- threads and `std::thread`
- atomics (`AtomicBool`, `AtomicUsize`, etc.)
- mutexes and condvars
- unsafe (Chase-Lev deque internals)

No async, no Tokio. Just `std` + `crossbeam` + `parking_lot`.

## Build

```bash
cargo build
cargo test
cargo bench
```

## License

MIT
