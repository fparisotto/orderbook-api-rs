CREATE TABLE orderbook_event (
    ts TIMESTAMP NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('buy', 'sell', 'fill', 'cancel')),
    order_id TEXT NOT NULL,
    order_quantity INTEGER,
    order_price NUMERIC,
    counterpart_id TEXT,
    counterpart_quantity INTEGER,
    counterpart_price NUMERIC
);

CREATE INDEX idx_orderbook_event_ts ON orderbook_event (ts);

