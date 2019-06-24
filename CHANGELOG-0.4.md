<hr>

## [0.4.2](https://gitlab.com/resolutions/review/tree/0.4.2) (2019-06-24) 

### Added
- API documentation has been added [here](https://gitlab.com/resolutions/review/tree/0.4.2/docs). Please refer to the documentation for changes on endpoints.

### Changed
- Environment variable `ETCD_SIG_KEY` is no longer needed. REview dynamically generates etcd keys as `benign_signatures_<kafka_topic_name>`.

### Fixed
- Fix a bug where a cluster insertion/update to database may fail.

<hr>

## [0.4.1](https://gitlab.com/resolutions/review/tree/0.4.1) (2019-05-20) 

### Changed
- Rename `Events` table to `Clusters` table (Note that database files created by REview 0.4.0 do not work with REview 0.4.1 and vice versa)
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

<hr>


## [0.4.0](https://gitlab.com/resolutions/review/tree/0.4.0) (2019-04-16)

### Added
- New fields are added in database schema. Thus REview 0.4.0 does not work with databases created by older version of REview.
    - `data_source` field in `Events` table
    - `size` and `event_ids` fields in `Outlier` table
- `GET /api/cluster` and `GET /api/outlier` return data with newly added fields.

### Changed

- REview server mode no longer reads configuration files. Instead the following environment variables needs to be set before running:
    - DATABASE_URL
    - REVIEWD_ADDR
    - DOCKER_HOST_IP
    - ETCD_ADDR
    - ETCD_SIG_KEY

### Other
- Docker-compose file to run REview server mode is added to [REsolutions repo](https://gitlab.com/resolutions/resolutions/tree/master/docker/reviewd).

<hr>
