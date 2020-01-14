CREATE TABLE category (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL
);
INSERT INTO category (name) VALUES('Non-Specified Alert');

CREATE TABLE data_source (
  id SERIAL PRIMARY KEY,
  topic_name TEXT NOT NULL,
  data_type TEXT NOT NULL,
  UNIQUE (topic_name)
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
  category_id INTEGER NOT NULL DEFAULT 1 REFERENCES category(id),
  detector_id INTEGER NOT NULL,
  event_ids NUMERIC(20, 0)[],
  qualifier_id INTEGER NOT NULL DEFAULT 2 REFERENCES qualifier(id),
  status_id INTEGER NOT NULL DEFAULT 2 REFERENCES Status(id),
  signature TEXT NOT NULL,
  size NUMERIC(20, 0) NOT NULL,
  score FLOAT8,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (cluster_id, data_source_id)
);

CREATE TABLE outlier (
  id SERIAL PRIMARY KEY,
  raw_event BYTEA NOT NULL,
  hashed_raw_event BYTEA NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  event_ids NUMERIC(20, 0)[] NOT NULL,
  size NUMERIC(20, 0) NOT NULL,
  UNIQUE (hashed_raw_event, data_source_id)
);

CREATE TABLE indicator (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  token JSONB NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  last_modification_time TIMESTAMP,
  UNIQUE (name)
);

CREATE TABLE description_element_type (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL
);
INSERT INTO description_element_type VALUES(1,'Int');
INSERT INTO description_element_type VALUES(2,'Enum');
INSERT INTO description_element_type VALUES(3,'Float');
INSERT INTO description_element_type VALUES(4,'Text');
INSERT INTO description_element_type VALUES(5,'IpAddr');
INSERT INTO description_element_type VALUES(6,'DateTime');

CREATE TABLE column_description (
  id SERIAL PRIMARY KEY,
  cluster_id INTEGER NOT NULL REFERENCES cluster (id),
  first_event_id TEXT NOT NULL,
  last_event_id TEXT NOT NULL,
  column_index INTEGER NOT NULL,
  type_id INTEGER NOT NULL REFERENCES description_element_type (id),
  count BIGINT NOT NULL,
  unique_count BIGINT NOT NULL,
  UNIQUE(cluster_id, first_event_id, last_event_id, column_index)
);

CREATE TABLE description_int (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  min BIGINT,
  max BIGINT,
  mean DOUBLE PRECISION,
  s_deviation DOUBLE PRECISION,
  mode BIGINT
);

CREATE TABLE description_enum (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  mode TEXT
);

CREATE TABLE description_float (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  min DOUBLE PRECISION,
  max DOUBLE PRECISION,
  mean DOUBLE PRECISION,
  s_deviation DOUBLE PRECISION,
  mode_smallest DOUBLE PRECISION,
  mode_largest DOUBLE PRECISION
);

CREATE TABLE description_text (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  mode TEXT
);

CREATE TABLE description_ipaddr (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  mode TEXT
);

CREATE TABLE description_datetime (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  mode TIMESTAMP
);

CREATE TABLE top_n_int (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  ranking BIGINT,
  value BIGINT,
  count BIGINT,
  UNIQUE(description_id, ranking)
);

CREATE TABLE top_n_enum (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  ranking BIGINT,
  value TEXT,
  count BIGINT,
  UNIQUE(description_id, ranking)
);

CREATE TABLE top_n_float (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  ranking BIGINT,
  value_smallest DOUBLE PRECISION,
  value_largest DOUBLE PRECISION,
  count BIGINT,
  UNIQUE(description_id, ranking)
);

CREATE TABLE top_n_text (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  ranking BIGINT,
  value TEXT,
  count BIGINT,
  UNIQUE(description_id, ranking)
);

CREATE TABLE top_n_ipaddr (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  ranking BIGINT,
  value TEXT,
  count BIGINT,
  UNIQUE(description_id, ranking)
);

CREATE TABLE top_n_datetime (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  ranking BIGINT,
  value TIMESTAMP,
  count BIGINT,
  UNIQUE(description_id, ranking)
);
