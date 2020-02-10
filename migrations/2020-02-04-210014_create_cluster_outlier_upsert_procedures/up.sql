/******************************************************
 * INSERT DATA SOURCE
 *
 * insert a new topic name into data_source table
 * Return id of inserted row
 ******************************************************/
CREATE OR REPLACE FUNCTION insert_data_source(
  topic_name VARCHAR,
  data_type VARCHAR
)
RETURNS INTEGER AS
$$
DECLARE
  new_data_source_id INTEGER;
BEGIN
  INSERT INTO data_source
    (topic_name, data_type)
  VALUES
    ($1, $2)
  RETURNING data_source.id INTO new_data_source_id;
  RETURN new_data_source_id;
EXCEPTION
  WHEN unique_violation THEN
    SELECT id
    INTO new_data_source_id
    FROM data_source
    WHERE data_source.topic_name = $1
    LIMIT 1;
  RETURN new_data_source_id;
END;
$$ LANGUAGE plpgsql;

/******************************************************
 * ATTEMPT CLUTER UPSERT
 *
 * attemp to upsert a cluster
 * return the number of rows updated (0 or 1)
 ******************************************************/
CREATE OR REPLACE FUNCTION attempt_cluster_upsert(
  max_event_id_num NUMERIC,
  clusterid VARCHAR,
  topic_name VARCHAR,
  data_type VARCHAR,
  detector_id INTEGER,
  event_ids NUMERIC(20, 0)[] DEFAULT NULL,
  signature VARCHAR DEFAULT NULL,
  score FLOAT8 DEFAULT NULL,
  size NUMERIC(20, 0) DEFAULT NULL
)
RETURNS INTEGER AS
$$
DECLARE
  current_signature VARCHAR;
  current_data_source_id INTEGER;
  current_event_ids NUMERIC(20, 0)[];
  event_id NUMERIC(20, 0);
  current_size NUMERIC(20, 0);
  current_score FLOAT8;
  new_id INTEGER;
BEGIN

  SELECT id
  INTO current_data_source_id
  FROM data_source
  WHERE data_source.topic_name = $3
  LIMIT 1;

  IF current_data_source_id IS NULL THEN
    SELECT * 
    INTO current_data_source_id
    FROM insert_data_source($3, $4);
  END IF;

  SELECT
    cluster.signature, cluster.event_ids, cluster.size, cluster.score
  INTO
    current_signature, current_event_ids, current_size, current_score
  FROM cluster
  WHERE cluster.cluster_id = $2
    and cluster.data_source_id = current_data_source_id
  LIMIT 1;

  IF current_size IS NOT NULL THEN
    IF $6 IS NULL THEN
      RETURN 0;
    END IF;
  END IF;

  IF $7 IS NULL THEN
    IF current_signature IS NULL THEN
      current_signature := '-';
    END IF;
  ELSE
    current_signature := $7;
  END IF;

  IF current_size IS NOT NULL THEN
    IF $9 IS NOT NULL THEN
      current_size := current_size + $9;
    END IF;
  ELSE 
    IF $9 IS NOT NULL THEN
      current_size := $9;
    ELSE
      current_size := 1;
    END IF;
  END IF;

  IF $6 IS NOT NULL THEN
    IF $9 >= $1 THEN
      IF current_event_ids IS NOT NULL THEN
        FOREACH event_id in ARRAY current_event_ids
        LOOP
          DELETE FROM event
            WHERE message_id = event_id 
            AND data_source_id = current_data_source_id;
        END LOOP;
      END IF;
      current_event_ids := $6;
    ELSEIF current_event_ids IS NOT NULL THEN
      current_event_ids := array_cat($6, current_event_ids);
      IF array_length(current_event_ids, 1) > $1 THEN
        LOOP
          EXECUTE
            'SELECT MIN(i) FROM UNNEST($1) i'
          INTO event_id
          USING current_event_ids;
          current_event_ids := array_remove(current_event_ids, event_id);
            DELETE FROM event
              WHERE message_id = event_id 
              AND data_source_id = current_data_source_id;
          IF array_length(current_event_ids, 1) = $1 THEN
            EXIT;
          END IF;
        END LOOP;
      END IF;
    ELSE
      current_event_ids := $6;
    END IF;
  END IF;

  INSERT INTO cluster (
    cluster_id,
    detector_id, 
    event_ids,
    signature,
    size,
    score,
    data_source_id,
    last_modification_time)
  VALUES
    ($2, $5, $6, current_signature, current_size, $8, current_data_source_id, NULL)
  ON CONFLICT (cluster_id, data_source_id)
  DO UPDATE
    SET
      signature = current_signature,
      event_ids = current_event_ids,
      size = current_size,
      score = current_score,
      last_modification_time = CURRENT_TIMESTAMP(0) at time zone 'UTC';

  RETURN 1;
END;
$$ LANGUAGE plpgsql;

/******************************************************
 * ATTEMPT OUTLIER UPSERT
 *
 * attemp to insert or update an outlier
 * return the number of rows updated (0 or 1)
 ******************************************************/
CREATE OR REPLACE FUNCTION attempt_outlier_upsert(
  max_event_id_num NUMERIC,
  o_id INTEGER,
  raw_event BYTEA,
  topic_name VARCHAR,
  data_type VARCHAR,
  event_ids NUMERIC(20, 0)[],
  size NUMERIC
)
RETURNS INTEGER AS
$$
DECLARE
  _data_source_id INTEGER;
  _event_ids NUMERIC(20, 0)[];
  _event_id NUMERIC(20, 0);
BEGIN
  IF array_length($6, 1) = 0 THEN
    RETURN 0;
  END IF;

  SELECT id
  INTO _data_source_id
  FROM data_source
  WHERE data_source.topic_name = $4
  LIMIT 1;

  IF _data_source_id IS NULL THEN
    SELECT * 
    INTO _data_source_id
    FROM insert_data_source($4, $5);
  END IF;

  IF $2 = 0 THEN
    IF array_length($6, 1) > $1 THEN
      LOOP
        EXECUTE
          'SELECT MIN(i) FROM UNNEST($1) i'
        INTO _event_id
        USING $6;
        $6 := array_remove($6, _event_id);
        DELETE FROM event
          WHERE message_id = _event_id 
          AND data_source_id = _data_source_id;
        IF array_length($6, 1) = $1 THEN
          EXIT;
        END IF;
      END LOOP;
    END IF;
    INSERT INTO outlier
        (raw_event, data_source_id, event_ids, size)
        VALUES
        ($3, _data_source_id, $6, $7);
  ELSE 
    SELECT outlier.event_ids
    INTO _event_ids
    FROM outlier
    WHERE outlier.id = $2
    LIMIT 1;

    IF _event_ids IS NULL THEN
      RETURN 0;
    END IF;

    _event_ids = array_cat($6, _event_ids);

    IF array_length(_event_ids, 1) > $1 THEN
      LOOP
        EXECUTE
          'SELECT MIN(i) FROM UNNEST($1) i'
        INTO _event_id
        USING _event_ids;
        _event_ids := array_remove(_event_ids, _event_id);
        DELETE FROM event
          WHERE message_id = _event_id 
          AND data_source_id = _data_source_id;
        IF array_length(_event_ids, 1) = $1 THEN
          EXIT;
        END IF;
      END LOOP;
    END IF;

    UPDATE outlier
      SET 
        event_ids = _event_ids,
        size = $7
      WHERE outlier.id = $2;
  END IF;
  RETURN 1;
END;
$$ LANGUAGE plpgsql;
