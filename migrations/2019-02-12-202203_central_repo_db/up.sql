CREATE TABLE Category (
  category_id INTEGER PRIMARY KEY,
  category TEXT NOT NULL
);
INSERT INTO Category VALUES(1,'Non-Specified Alert');

CREATE TABLE Clusters (
  id INTEGER PRIMARY KEY,
  cluster_id TEXT,
  category_id INTEGER NOT NULL,
  detector_id INTEGER NOT NULL,
  event_ids BLOB,
  raw_event_id INTEGER,
  qualifier_id INTEGER NOT NULL,
  status_id INTEGER NOT NULL,
  signature TEXT NOT NULL,
  size TEXT NOT NULL DEFAULT "1",
  score REAL,
  data_source_id INTEGER NOT NULL,
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (cluster_id, data_source_id) ON CONFLICT REPLACE,
  FOREIGN KEY(category_id) REFERENCES Category(category_id)
  FOREIGN KEY(data_source_id) REFERENCES DataSource(data_source_id)
  FOREIGN KEY(qualifier_id) REFERENCES Qualifier(qualifier_id)
  FOREIGN KEY(raw_event_id) REFERENCES RawEvent(raw_event_id)
  FOREIGN KEY(status_id) REFERENCES Status(status_id)
);

CREATE TABLE DataSource (
  data_source_id INTEGER NOT NULL PRIMARY KEY, 
  topic_name TEXT NOT NULL,
  data_type TEXT NOT NULL,
);

CREATE TABLE Outliers (
  id INTEGER PRIMARY KEY,
  raw_event BLOB NOT NULL,
  data_source_id INTEGER NOT NULL,
  event_ids BLOB,
  raw_event_id INTEGER,
  size TEXT,
  UNIQUE (raw_event, data_source_id) ON CONFLICT REPLACE,
  FOREIGN KEY(data_source_id) REFERENCES DataSource(data_source_id)
  FOREIGN KEY(raw_event_id) REFERENCES RawEvent(raw_event_id)
);

CREATE TABLE Qualifier (
  qualifier_id INTEGER PRIMARY KEY,
  qualifier TEXT NOT NULL
);
INSERT INTO Qualifier VALUES(1,'benign');
INSERT INTO Qualifier VALUES(2,'unknown');
INSERT INTO Qualifier VALUES(3,'suspicious');

CREATE TABLE RawEvent (
  raw_event_id INTEGER NOT NULL PRIMARY KEY,
  raw_event BLOB NOT NULL,
  data_source_id INTEGER NOT NULL,
  FOREIGN KEY(data_source_id) REFERENCES DataSource(data_source_id)
);

CREATE TABLE Status (
  status_id INTEGER PRIMARY KEY,
  status TEXT NOT NULL
);
INSERT INTO Status VALUES(1,'reviewed');
INSERT INTO Status VALUES(2,'pending review');
INSERT INTO Status VALUES(3,'disabled');
