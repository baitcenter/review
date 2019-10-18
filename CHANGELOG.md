# Changelog

This file documents all notable changes to this project. The format of this file
is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.2] - 2019-10-17

### Added

- REviewd outputs logs of processed HTTP Requests and Response like below:
```
[2019-10-12T01:36:58Z INFO  actix_web::middleware::logger] 10.0.170.135:54135 "PUT /api/cluster HTTP/1.1" 200 0 "-" "reqwest/0.9.21" 0.046250
```

- Added saving column descriptions for multidimensional clustering into db.

### Changed

- Fetching raw events from Kafka runs faster by running I/O operations in
  parallel with internal data udpates.

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
- Each cluster `examples` is a pair of a single `raw_event` and a list of `event_ids` like below:
```
"examples": {
            "raw_event": "something",
            "event_ids": [
                464118,
                465078
            ]
          }
```
- If a cluster is created only from training files, `raw_event` in `examples` will be `-`.

### Removed

- Removed REview file mode. REview is no longer able to read REmake output files.

### Fixed

- Corrected the error message for `DOCKER_HOST_IP` not set.
- Delete outliers from outliers table when REmake finds new clusters from them.

## [0.5.0] - 2019-07-24

### Added

- A new field called `score` is added to `Clusters` table to store score value. Note that database files created by an older version of REview are not compatible with this version.

### Changed

- The terminal client displays binary data in REmake output files.

## [0.4.2] - 2019-06-24

### Added

- API documentation has been added
  [here](https://gitlab.com/resolutions/review/tree/0.4.2/docs). Please refer to
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
```
        "cluster_id": "get http www",
        "detector_id": 2,
        "qualifier_id": 2,
        "status_id": 2,
```

v0.4.1
```
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
- Docker-compose file to run REview server mode is added to [REsolutions
  repo](https://gitlab.com/resolutions/resolutions/tree/master/docker/reviewd).

### Changed

- REview server mode no longer reads configuration files. Instead the following
  environment variables needs to be set before running:
  - `DATABASE_URL`
  - `REVIEWD_ADDR`
  - `DOCKER_HOST_IP`
  - `ETCD_ADDR`
  - `ETCD_SIG_KEY`

[0.6.2]: https://gitlab.com/resolutions/review/compare/0.6.1...0.6.2
[0.6.1]: https://gitlab.com/resolutions/review/compare/0.6.0...0.6.1
[0.6.0]: https://gitlab.com/resolutions/review/compare/0.5.0...0.6.0
[0.5.0]: https://gitlab.com/resolutions/review/compare/0.4.2...0.5.0
[0.4.2]: https://gitlab.com/resolutions/review/compare/0.4.1...0.4.2
[0.4.1]: https://gitlab.com/resolutions/review/compare/0.4.0...0.4.1
[0.4.0]: https://gitlab.com/resolutions/review/compare/0.3.9...0.4.0
