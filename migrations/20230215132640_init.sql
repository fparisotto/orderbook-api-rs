CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE EVENT_TYPE_ENUM AS ENUM ('buy', 'sell', 'fill', 'cancel');

CREATE TABLE orderbook_event (
    ts TIMESTAMPTZ NOT NULL,
    event_type EVENT_TYPE_ENUM NOT NULL,
    order_id UUID NOT NULL,
    order_quantity INT,
    order_price NUMERIC,
    counterpart_id UUID,
    counterpart_quantity INT,
    counterpart_price NUMERIC
);

CREATE INDEX idx_orderbook_event_ts ON orderbook_event (ts);
