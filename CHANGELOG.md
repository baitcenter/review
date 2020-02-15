# Changelog

This file documents all notable changes to this project. The format of this file
is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A new endpoint `GET /api/template` to fetch template(s) from template table.
- A new endpoint `POST /api/template` to create a new template.

### Changed

- Parameters on the `PUT /api/description` endpoint has changed.
  - Add a query for the topic name of `data_source`.
- The request body for `PUT /api/outlier` has been changed to include `id` and
  `size`. 

### Fixed

- Since only cluster ids had been searched for to add new descriptions, 
  the new ones had not been properly created. By checking the topic name of
  clusters whose column statistics are to be created, it's fixed.

### Removed

- Subcommands on the `review` command line is removed.

## [0.7.0] - 2020-01-28

### Added

- `event` and `kafka_metadata` tables.
- A new endpoint `GET /api/event/search` to fetch raw_events from event table.
- Periodic tasks to fetch kafka metadata and raw events that work with the
  environment variables below. `raw_events` corresponding to `event_ids` in
  `cluster` and `outlier` tables are stored in `event` table by these periodic
  tasks.
  - `MAX_OFFSET_COUNT`: Maximum number of offsets for each kafka topic to fetch
    per task. Default value is 1000.
  - `TASK_TIME_INTERVAL`: Periodic task interval (in seconds). Default value is
    900 (15 minutes). If the value is 0, the periodic task will not be
    initiated.
- A new environment variable `MAX_EVENT_ID_NUM` to configure the maximum number
  of `event_ids` per cluster and outlier.
- `select` query to `GET /api/outlier`.

### Changed

- The data type of `raw_event` column has been changed from `TEXT` to `BYTEA`.
  `bytea_output` in Postgres server configuration must be its default setting of
  `hex`.
- There are changes in the values returned when fetching clusters and outliers.
  Please fetch events corresponding to `event_ids` to display human-readable
  raw_events.
  - `raw_event` in cluster is no longer displayed.
  - `outlier` in outlier is displayed as String of hex values.
- No longer need `DOCKER_HOST_IP` and `ETCD_ADDR` environment variables.

### Fixed

- Inserting outliers containing NULL(0x00) character fails.
- May not insert cluster when `data_source::add` function is called
  simultaneously.

### Removed

- `raw_event` table
- `PUT /api/etcd/suspicious_token` endpoint

## [0.6.3] - 2019-12-30

### Added

- Added new endpoints for `data_source` and `indicator` tables.

### Changed

- Enum values are stored as `String` type of raw data instead of their mapped
  unsigned integers.

### Deprecated

- Subcommands on the `review` command line is deprecated. Simply running
  `review` is preferred to running `review reviewd`. The subcommand is currently
  ignored silently, but will become an error in 0.8.0.

### Removed

- The `client` subcommand was obsoleted by REsolutions-frontend.
- Removed `POST /api/cluster/` and `POST /api/cluster` endpoints.

### Fixed

- Each HTTP requests uses at most one database connection. This removes database
  timeouts caused by connection exhaustion.

## [0.6.2] - 2019-11-08

### Added

- REviewd outputs logs of processed HTTP Requests and Response like below:

```
[2019-10-12T01:36:58Z INFO  actix_web::middleware::logger] 10.0.170.135:54135 "PUT /api/cluster HTTP/1.1" 200 0 "-" "reqwest/0.9.21" 0.046250
```

- Added saving column descriptions for multidimensional clustering into db.
- Can designated the frontend app directory to be served through HTTP.
- Added pagination and ordering for cluster and outlier tables. Please refer to
  the API documentation for details.

### Changed

- Fetching raw events from Kafka runs faster by running I/O operations in
  parallel with internal data udpates.

- The number of clusters/outliers returned per request is 10 by default.

- The format of response body has changed:
  - `examples` field, which consists of `raw_event` and `event_ids` fields, has
    been removed. `raw_event` and `event_ids` fields are displayed separately.
  - Field names of clusters/outliers in response body are in alphabetical order
    like below:

  ```json
  {
      "category": "Non-Specified Alert",
      "cluster_id": "f-14-f",
      "data_source": "log01",
      "detector_id": 1,
      "event_ids": [
          281474977755983,
          281474977760773
      ],
      "last_modification_time": null,
      "qualifier": "unknown",
      "raw_event": "Error: username or password error,please input again.\r",
      "score": null,
      "signature": "(?i)(yadm.*x=di.*yadm)",
      "size": "10",
      "status": "pending review"
  }
  ```

### Removed

- Removed `GET /api/cluster/search` and `GET /api/outlier/{data_source}`
  endpoints.

## [0.6.1] - 2019-10-08

### Fixed

- Fixed a bug where `raw_event` in `examples` might be displayed as `-` even if
  the clusters were not created from training files.
- Fixed build on macOS Catalina.

## [0.6.0] - 2019-09-27

### Added

- Added new tables called `RawEvents` and `DataSource` in database schema.
- Added kafka consumer to fetch messages directly from Kafka for raw events. 
  The environment variable, KAFKA_URL, must be set to run REviewd.

### Changed

- The underlying database has changed from SQLite to PostgreSQL.
- Each cluster `examples` is a pair of a single `raw_event` and a list of
  `event_ids` like below:

```json
"examples": {
            "raw_event": "something",
            "event_ids": [
                464118,
                465078
            ]
          }
```

- If a cluster is created only from training files, `raw_event` in `examples`
  will be `-`.

### Removed

- Removed REview file mode. REview is no longer able to read REmake output files.

### Fixed

- Corrected the error message for `DOCKER_HOST_IP` not set.
- Delete outliers from outliers table when REmake finds new clusters from them.

## [0.5.0] - 2019-07-24

### Added

- A new field called `score` is added to `Clusters` table to store score value.
  Note that database files created by an older version of REview are not
  compatible with this version.

### Changed

- The terminal client displays binary data in REmake output files.

## [0.4.2] - 2019-06-24

### Added

- API documentation has been added
  [here](https://github.com/petabi/review/tree/0.4.2/docs). Please refer to
  the documentation for changes on endpoints.

### Changed

- Environment variable `ETCD_SIG_KEY` is no longer needed. REview dynamically
  generates etcd keys as `benign_signatures_<kafka_topic_name>`.

### Fixed

- Fix a bug where a cluster insertion/update to database may fail.

## [0.4.1] - 2019-05-20

### Changed

- Rename `Events` table to `Clusters` table (Note that database files created by
  REview 0.4.0 do not work with REview 0.4.1 and vice versa)
- The format of responses from `GET /api/cluster` endpoint has changed:
  - return actual values of status and qualifier.
  - return the value of category.
  - return examples with property names.

v0.4.0

```json
        "cluster_id": "get http www",
        "detector_id": 2,
        "qualifier_id": 2,
        "status_id": 2,
```

v0.4.1

```json
        "cluster_id": "get http www",
        "detector_id": 2,
        "qualifier": "unknown",
        "status": "pending review",
        "category": "Non-Specified Alert",
        "signature": "(?i)(^get.+http.+www)",
        "data_source": "log01",
        "size": 218,
        "examples": [
            {
                "id": 1,
                "raw_event": "raw_event"
            },
```

### Fixed

- REview http client mode now works with REviewd 0.4.1.

## [0.4.0] - 2019-04-16

### Added

- New fields are added in database schema. Thus REview 0.4.0 does not work with
  databases created by older version of REview.
  - `size` and `event_ids` fields in `Outlier` table
  - `data_source` field in `Events` table
- `GET /api/cluster` and `GET /api/outlier` return data with newly added fields.
- Docker-compose file to run REview server mode is added to REsolutions
  repo.

### Changed

- REview server mode no longer reads configuration files. Instead the following
  environment variables needs to be set before running:
  - `DATABASE_URL`
  - `REVIEWD_ADDR`
  - `DOCKER_HOST_IP`
  - `ETCD_ADDR`
  - `ETCD_SIG_KEY`

[Unreleased]: https://github.com/petabi/review/compare/0.7.0...master
[0.7.0]: https://github.com/petabi/review/compare/0.6.3...0.7.0
[0.6.3]: https://github.com/petabi/review/compare/0.6.2...0.6.3
[0.6.2]: https://github.com/petabi/review/compare/0.6.1...0.6.2
[0.6.1]: https://github.com/petabi/review/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/petabi/review/compare/0.5.0...0.6.0
[0.5.0]: https://github.com/petabi/review/compare/0.4.2...0.5.0
[0.4.2]: https://github.com/petabi/review/compare/0.4.1...0.4.2
[0.4.1]: https://github.com/petabi/review/compare/0.4.0...0.4.1
[0.4.0]: https://github.com/petabi/review/compare/0.3.9...0.4.0
