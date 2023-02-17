# OrderBook API

A [Rust](https://www.rust-lang.org/) implementation of Order Book manager
system.

_This is a toy project of Rest API [Rust](https://www.rust-lang.org/)
application, built for learning proposes, not intended to be used in
production._

## Brief explanation of system design

- The _Order Book_ state is managed by a _single future_, scheduled to run in
  [tokio](https://tokio.rs/).
- Every interaction on the _Order Book_ state in done in memory.
- Each user request are represented by a _Command_, this _Command_ is evaluated
  by the _Order Book_ logic, changing its sate and emitting _Event_'s.
- _Event_'s are persisted in the data store, only after persisting the _Event_
  the API returns a response to the user.
- This resembles an [actor model](https://en.wikipedia.org/wiki/Actor_model)
  design, with a persistent state.

## Missing features

- User authentication and balance checking.
- Restore the _Order Book_ state from data store events.
- How: replay all _Event_'s in order of its occurrence and applying the state
  changes.
- Periodically take a snapshot of the _Order Book_ state to speedup the restore
  process.
- How: first load the last snapshot, then replay all _Event_'s after.
- Deny requests (503) on heavy load.
- How: checking if any _Command_ is dropped.

## How to run

```bash
$ docker compose down --volumes && docker compose build && docker compose up
```

## How to run load test

You need [drill](https://github.com/fcsonline/drill), use `cargo` to install it.

```bash
$ cargo install drill
```

With the `docker-compose.yml` up, run:

```bash
$ drill --benchmark drill.yml --stats --quiet
```
