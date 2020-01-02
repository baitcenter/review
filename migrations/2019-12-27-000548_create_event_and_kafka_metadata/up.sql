CREATE TABLE event (
  id SERIAL PRIMARY KEY,
  message_id NUMERIC(20, 0) NOT NULL,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  raw_event TEXT,
  UNIQUE (message_id, data_source_id)
);

CREATE TABLE kafka_metadata (
  id SERIAL PRIMARY KEY,
  data_source_id INTEGER NOT NULL REFERENCES data_source(id),
  partition INTEGER NOT NULL,
  offsets BIGINT NOT NULL,
  message_ids NUMRANGE NOT NULL,
  UNIQUE (data_source_id, partition, offsets)
);

/******************************************************
 * LOOKUP EVENTS WITH NO RAW EVENT
 *
 * Return a list of message_id of events with
 * no raw_event 
 ******************************************************/
CREATE OR REPLACE FUNCTION lookup_events_with_no_raw_event(
  data_source_id INTEGER
)
RETURNS SETOF NUMERIC AS
$$
DECLARE
  message_id_range NUMRANGE;
  lower_message_id NUMERIC;
  upper_message_id NUMERIC;
BEGIN
  SELECT 
    NUMRANGE(MIN(LOWER(message_ids)), MAX(UPPER(message_ids))+1) message_ids
  INTO message_id_range
  FROM (
    SELECT 
      kafka_metadata.message_ids 
    FROM 
      kafka_metadata 
    WHERE kafka_metadata.data_source_id = $1
  ) a;

IF lower_inc(message_id_range) THEN
  lower_message_id := lower(message_id_range);
ELSEIF lower_inf(message_id_range) IS FALSE THEN
  lower_message_id := lower(message_id_range) + 1;
END IF;

IF upper_inc(message_id_range) THEN
  upper_message_id := upper(message_id_range);
ELSEIF upper_inf(message_id_range) IS FALSE THEN
  upper_message_id := upper(message_id_range) - 1;
END IF;

RETURN QUERY
SELECT message_id
FROM event
WHERE
  event.message_id BETWEEN lower_message_id AND upper_message_id
  AND event.data_source_id = $1
  AND event.raw_event IS NULL;
END;
$$ LANGUAGE plpgsql;

/******************************************************
 * LOOKUP KAFKA METADATA
 *
 * Return Kafka metadata which contains the specified 
 * message_id or NULL.
 ******************************************************/
CREATE OR REPLACE FUNCTION lookup_kafka_metadata(
  data_source_id INTEGER,
  message_id NUMERIC
)
RETURNS JSONB AS
$$
DECLARE
  json_blob JSONB;
BEGIN
  SELECT to_jsonb(a)
  INTO json_blob
  FROM (
    SELECT 
      kafka_metadata.offsets, 
      kafka_metadata.partition, 
      UPPER(kafka_metadata.message_ids) as message_id
    FROM kafka_metadata
    WHERE LOWER(kafka_metadata.message_ids) <= $2 
          AND UPPER(kafka_metadata.message_ids) >= $2
          AND kafka_metadata.data_source_id = $1
  ) a;

  RETURN json_blob;
END;
$$ LANGUAGE plpgsql;
