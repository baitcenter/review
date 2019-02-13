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

-- Status is the current system status of the cluster. --
-- Changes in status may warrant publication, or republication --
-- of a rule. It also offers a means to turn on or off --
-- a particular rule.--
CREATE TABLE Status (
  status_id INTEGER PRIMARY KEY,
  status TEXT NOT NULL
);
INSERT INTO Status VALUES(1,'active');
INSERT INTO Status VALUES(2,'review');
INSERT INTO Status VALUES(3,'disabled');

-- Detectors define the processing in the system. --
-- In particular they define the input that is examined --
CREATE TABLE Detectors (
  detector_id INTEGER PRIMARY KEY,
  detector_type INTEGER NOT NULL,
  description TEXT,
  input TEXT,
  local_db TEXT,
  output TEXT NOT NULL,
  status_id INTEGER NOT NULL,
  suspicious_tokens TEXT,
  target_detector_id INTEGER DEFAULT -1,
  time_regex TEXT,
  time_format TEXT,
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(status_id) REFERENCES Status(status_id) ON UPDATE CASCADE
);
INSERT INTO Detectors VALUES(1, 0, 'Local Correlator', '', '', 'events_out.log', 1, '', 1, '', '', datetime('now'));

-- Events represent rules that have been crafted from particular --
-- clusters. --
CREATE TABLE Events (
  event_id INTEGER PRIMARY KEY,
  cluster_id TEXT,
  description TEXT,
  category_id INTEGER NOT NULL,
  detector_id INTEGER NOT NULL,
  examples TEXT,
  priority_id INTEGER NOT NULL,
  qualifier_id INTEGER NOT NULL,
  status_id INTEGER NOT NULL,
  rules TEXT,
  signature TEXT NOT NULL,
  last_modification_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(category_id) REFERENCES Category(category_id) ON UPDATE CASCADE,
  FOREIGN KEY(detector_id) REFERENCES Detectors(detector_id) ON UPDATE CASCADE,
  FOREIGN KEY(priority_id) REFERENCES Priority(priority_id) ON UPDATE CASCADE,
  FOREIGN KEY(qualifier_id) REFERENCES Qualifier(qualifier_id) ON UPDATE CASCADE,
  FOREIGN KEY(status_id) REFERENCES Status(status_id) ON UPDATE CASCADE
);

-- The Ready_table is used as a staging area for indicating what
-- rules are pushed out as well as a log of those rules that have
-- been published.--
CREATE TABLE Ready_table (
  publish_id INTEGER PRIMARY KEY,
  action_id INTEGER NOT NULL,
  event_id INTEGER,
  detector_id INTEGER,
  time_published DATETIME DEFAULT 0,
  FOREIGN KEY(action_id) REFERENCES Action(action_id) ON UPDATE CASCADE,
  FOREIGN KEY(event_id) REFERENCES Events(event_id) ON UPDATE CASCADE ON DELETE CASCADE,
  FOREIGN KEY(detector_id) REFERENCES Detectors(detector_id) ON UPDATE CASCADE ON DELETE CASCADE
);
