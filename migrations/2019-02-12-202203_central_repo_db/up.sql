CREATE TABLE Category (
  category_id INTEGER PRIMARY KEY,
  category TEXT NOT NULL
);
INSERT INTO Category VALUES(1,'Non-Specified Alert');

CREATE TABLE DataSource (
  data_source_id INTEGER NOT NULL PRIMARY KEY, 
  topic_name TEXT NOT NULL,
  data_type TEXT NOT NULL
);

CREATE TABLE RawEvent (
  raw_event_id INTEGER NOT NULL PRIMARY KEY,
  raw_event BYTEA NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES DataSource(data_source_id)
);

CREATE TABLE Qualifier (
  qualifier_id INTEGER PRIMARY KEY,
  qualifier TEXT NOT NULL
);
INSERT INTO Qualifier VALUES(1,'benign');
INSERT INTO Qualifier VALUES(2,'unknown');
INSERT INTO Qualifier VALUES(3,'suspicious');

CREATE TABLE Status (
  status_id INTEGER PRIMARY KEY,
  status TEXT NOT NULL
);
INSERT INTO Status VALUES(1,'reviewed');
INSERT INTO Status VALUES(2,'pending review');
INSERT INTO Status VALUES(3,'disabled');

CREATE TABLE Clusters (
  id INTEGER PRIMARY KEY,
  cluster_id TEXT,
  category_id INTEGER NOT NULL REFERENCES Category(category_id),
  detector_id INTEGER NOT NULL,
  event_ids BYTEA,
  raw_event_id INTEGER REFERENCES RawEvent(raw_event_id),
  qualifier_id INTEGER NOT NULL REFERENCES Qualifier(qualifier_id),
  status_id INTEGER NOT NULL REFERENCES Status(status_id),
  signature TEXT NOT NULL,
  size TEXT NOT NULL DEFAULT '1',
  score REAL,
  data_source_id INTEGER NOT NULL REFERENCES DataSource(data_source_id),
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE Outliers (
  id INTEGER PRIMARY KEY,
  raw_event BYTEA,
  data_source_id INTEGER NOT NULL REFERENCES DataSource(data_source_id),
  event_ids BYTEA NOT NULL,
  raw_event_id INTEGER REFERENCES RawEvent(raw_event_id),
  size TEXT
);
