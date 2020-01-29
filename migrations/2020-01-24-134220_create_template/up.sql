CREATE TABLE template (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    method TEXT NOT NULL,
    algorithm TEXT,
    min_token_length BIGINT,
    eps DOUBLE PRECISION,
    format JSONB,
    dimension_default BIGINT,
    dimensions BIGINT[],
    UNIQUE(name)
);
