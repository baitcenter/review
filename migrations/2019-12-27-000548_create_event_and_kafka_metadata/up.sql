CREATE TABLE event (
  id SERIAL PRIMARY KEY,
  message_id NUMERIC(20, 0) NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  raw_event TEXT,
  UNIQUE (message_id, data_source_id)
);

CREATE TABLE kafka_metadata (
  id SERIAL PRIMARY KEY,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  partition INTEGER NOT NULL,
  offsets BIGINT NOT NULL,
  message_ids NUMRANGE NOT NULL,
  UNIQUE (data_source_id, partition, offsets)
);
