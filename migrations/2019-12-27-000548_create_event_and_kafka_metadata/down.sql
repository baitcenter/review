DROP TABLE kafka_metadata;
DROP TABLE event;
DROP FUNCTION IF EXISTS lookup_events_with_no_raw_event(INTEGER);
DROP FUNCTION IF EXISTS lookup_kafka_metadata(INTEGER, NUMERIC);
DROP FUNCTION IF EXISTS attempt_event_ids_update(NUMERIC);
