# My very cool payment processor

## Code documentation is mostly Rust-docs style

- No fancy parallel processing or async, just a simple engine that gets the job done
- The payment engine itself is decoupled from the I/O, it doesn't care where the transactions come from - it just processes Transaction structs
- Streams the document and processes as it reads, no upfront loading

## Whiteboard Discussion

- Main issue is that the engine is stateful, and the state is in memory - this obviously is a problem when dealing with massive data sets and is where the architecture falls aparat.
- In low latency scenarios (these are always fun to think about!) where we need to process the data as fast as possible, we would have a producer/consumer model with two tighly controlled threads, one that reads the document and the other one that processes it. Crossbeam channels being the obvious choice here. I'll probably explore this solution myself in the future to satisfy my own curiosity.
