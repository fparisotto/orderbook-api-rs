# orderbook-api-rs

TODO

## Brief explanation of system design

TODO

## How to run

```bash
$ docker compose down --volumes && docker compose build && docker compose up
```

## How to run load test

You need [drill](https://github.com/fcsonline/drill), use `cargo` to install.

```bash
$ cargo install drill
```

With the `docker-compose.yml` up, run:

```bash
$ drill --benchmark drill.yml --stats --quiet
```
