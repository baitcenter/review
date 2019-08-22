CREATE TABLE Action (
  action_id INTEGER PRIMARY KEY,
  action TEXT NOT NULL
);
INSERT INTO Action VALUES(1,'Update');
INSERT INTO Action VALUES(2,'Delete');

-- Category is the subjective characterization of a cluster. --
-- It is expected that the user will tailor this to system needs. --
CREATE TABLE Category (
  category_id INTEGER PRIMARY KEY,
  category TEXT NOT NULL
);
INSERT INTO Category VALUES(1,'Non-Specified Alert');

CREATE TABLE Clusters (
  id INTEGER PRIMARY KEY,
  cluster_id TEXT,
  description TEXT,
  category_id INTEGER NOT NULL,
  detector_id INTEGER NOT NULL,
  examples BLOB,
  priority_id INTEGER NOT NULL,
  qualifier_id INTEGER NOT NULL,
  status_id INTEGER NOT NULL,
  rules TEXT,
  signature TEXT NOT NULL,
  size TEXT NOT NULL DEFAULT "1",
  score REAL,
  data_source_id INTEGER NOT NULL,
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (cluster_id, data_source_id) ON CONFLICT REPLACE,
  FOREIGN KEY(category_id) REFERENCES Category(category_id) ON UPDATE CASCADE,
  FOREIGN KEY(data_source_id) REFERENCES DataSource(data_source_id) ON UPDATE CASCADE,
  FOREIGN KEY(priority_id) REFERENCES Priority(priority_id) ON UPDATE CASCADE,
  FOREIGN KEY(qualifier_id) REFERENCES Qualifier(qualifier_id) ON UPDATE CASCADE,
  FOREIGN KEY(status_id) REFERENCES Status(status_id) ON UPDATE CASCADE
);

CREATE TABLE DataSource (
  data_source_id INTEGER NOT NULL PRIMARY KEY, 
  topic_name TEXT NOT NULL,
  data_type TEXT NOT NULL,
  UNIQUE (topic_name, data_type) ON CONFLICT REPLACE
);

CREATE TABLE Outliers (
  id INTEGER PRIMARY KEY,
  raw_event BLOB NOT NULL,
  data_source_id INTEGER NOT NULL,
  event_ids BLOB,
  size TEXT,
  UNIQUE (raw_event, data_source_id) ON CONFLICT REPLACE,
  FOREIGN KEY(data_source_id) REFERENCES DataSource(data_source_id) ON UPDATE CASCADE
);

-- Qualifier is the qualification of a cluster (i.e. good or bad). --
CREATE TABLE Qualifier (
  qualifier_id INTEGER PRIMARY KEY,
  qualifier TEXT NOT NULL
);
INSERT INTO Qualifier VALUES(1,'benign');
INSERT INTO Qualifier VALUES(2,'unknown');
INSERT INTO Qualifier VALUES(3,'suspicious');

-- Priority is the subjective priority to attach to events that --
-- match a partiuclar cluster. --
CREATE TABLE Priority (
  priority_id INTEGER PRIMARY KEY,
  priority TEXT NOT NULL
);
INSERT INTO Priority VALUES(1,'low');
INSERT INTO Priority VALUES(2,'mid');
INSERT INTO Priority VALUES(3,'high');

CREATE TABLE RawEvent (
  event_id TEXT NOT NULL,
  raw_event BLOB NOT NULL,
  data_source_id INTEGER NOT NULL,
  PRIMARY KEY (event_id, data_source_id),
  UNIQUE (event_id, data_source_id) ON CONFLICT REPLACE
  FOREIGN KEY(data_source_id) REFERENCES DataSource(data_source_id) ON UPDATE CASCADE
);

-- Status is the current system status of the cluster. --
-- Changes in status may warrant publication, or republication --
-- of a rule. It also offers a means to turn on or off --
-- a particular rule.--
CREATE TABLE Status (
  status_id INTEGER PRIMARY KEY,
  status TEXT NOT NULL
);
INSERT INTO Status VALUES(1,'reviewed');
INSERT INTO Status VALUES(2,'pending review');
INSERT INTO Status VALUES(3,'disabled');
