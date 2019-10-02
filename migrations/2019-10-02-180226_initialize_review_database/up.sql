CREATE TABLE category (
  category_id SERIAL PRIMARY KEY,
  name TEXT NOT NULL
);
INSERT INTO category VALUES(1,'Non-Specified Alert');

CREATE TABLE data_source (
  data_source_id SERIAL PRIMARY KEY,
  topic_name TEXT NOT NULL,
  data_type TEXT NOT NULL,
  UNIQUE (topic_name)
);

CREATE TABLE raw_event (
  raw_event_id SERIAL PRIMARY KEY,
  data BYTEA NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(data_source_id)
);

CREATE TABLE qualifier (
  qualifier_id INTEGER PRIMARY KEY,
  description TEXT NOT NULL
);
INSERT INTO qualifier VALUES(1,'benign');
INSERT INTO qualifier VALUES(2,'unknown');
INSERT INTO qualifier VALUES(3,'suspicious');

CREATE TABLE status (
  status_id INTEGER PRIMARY KEY,
  description TEXT NOT NULL
);
INSERT INTO status VALUES(1,'reviewed');
INSERT INTO status VALUES(2,'pending review');
INSERT INTO status VALUES(3,'disabled');

CREATE TABLE clusters (
  id SERIAL PRIMARY KEY,
  cluster_id TEXT,
  category_id INTEGER NOT NULL REFERENCES category(category_id),
  detector_id INTEGER NOT NULL,
  event_ids BYTEA,
  raw_event_id INTEGER REFERENCES raw_event(raw_event_id),
  qualifier_id INTEGER NOT NULL REFERENCES qualifier(qualifier_id),
  status_id INTEGER NOT NULL REFERENCES Status(status_id),
  signature TEXT NOT NULL,
  size TEXT NOT NULL DEFAULT '1',
  score FLOAT8,
  data_source_id INTEGER NOT NULL REFERENCES data_source(data_source_id),
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (cluster_id, data_source_id)
);

CREATE TABLE outliers (
  id SERIAL PRIMARY KEY,
  raw_event BYTEA NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(data_source_id),
  event_ids BYTEA NOT NULL,
  raw_event_id INTEGER REFERENCES raw_event(raw_event_id),
  size TEXT,
  UNIQUE (raw_event, data_source_id)
);

CREATE TABLE indicator (
  id SERIAL PRIMARY KEY,
  description TEXT,
  source INTEGER REFERENCES data_source(data_source_id),
  category INTEGER REFERENCES category(category_id),
  qualification DOUBLE PRECISION
);

CREATE TABLE token (
  id SERIAL PRIMARY KEY,
  name BYTEA,
  indicator INTEGER REFERENCES indicator(id)
);
