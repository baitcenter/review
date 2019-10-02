CREATE TABLE category (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL
);
INSERT INTO category VALUES(1,'Non-Specified Alert');

CREATE TABLE data_source (
  id SERIAL PRIMARY KEY,
  topic_name TEXT NOT NULL,
  data_type TEXT NOT NULL,
  UNIQUE (topic_name)
);

CREATE TABLE raw_event (
  id SERIAL PRIMARY KEY,
  data BYTEA NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id)
);

CREATE TABLE qualifier (
  id INTEGER PRIMARY KEY,
  description TEXT NOT NULL
);
INSERT INTO qualifier VALUES(1,'benign');
INSERT INTO qualifier VALUES(2,'unknown');
INSERT INTO qualifier VALUES(3,'suspicious');

CREATE TABLE status (
  id INTEGER PRIMARY KEY,
  description TEXT NOT NULL
);
INSERT INTO status VALUES(1,'reviewed');
INSERT INTO status VALUES(2,'pending review');
INSERT INTO status VALUES(3,'disabled');

CREATE TABLE cluster (
  id SERIAL PRIMARY KEY,
  cluster_id TEXT,
  category_id INTEGER NOT NULL REFERENCES category(id),
  detector_id INTEGER NOT NULL,
  event_ids BYTEA,
  raw_event_id INTEGER REFERENCES raw_event(id),
  qualifier_id INTEGER NOT NULL REFERENCES qualifier(id),
  status_id INTEGER NOT NULL REFERENCES Status(id),
  signature TEXT NOT NULL,
  size TEXT NOT NULL DEFAULT '1',
  score FLOAT8,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (cluster_id, data_source_id)
);

CREATE TABLE outlier (
  id SERIAL PRIMARY KEY,
  raw_event BYTEA NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  event_ids BYTEA NOT NULL,
  raw_event_id INTEGER REFERENCES raw_event(id),
  size TEXT,
  UNIQUE (raw_event, data_source_id)
);

CREATE TABLE indicator (
  id SERIAL PRIMARY KEY,
  description TEXT,
  source INTEGER REFERENCES data_source(id),
  category INTEGER REFERENCES category(id),
  qualification DOUBLE PRECISION
);

CREATE TABLE token (
  id SERIAL PRIMARY KEY,
  name BYTEA,
  indicator INTEGER REFERENCES indicator(id)
);
