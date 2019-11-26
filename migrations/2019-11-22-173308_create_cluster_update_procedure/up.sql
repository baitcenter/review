/******************************************************
 * ATTEMPT CLUSTER UPDATE
 *
 * attempt to update a specific cluster.
 * Return the number of rows updated (0 or 1)
 ******************************************************/
CREATE OR REPLACE FUNCTION attempt_cluster_update(
  current_cluster VARCHAR,
  data_source_name VARCHAR,
  new_category VARCHAR DEFAULT NULL,
  new_cluster VARCHAR DEFAULT NULL,
  new_qualifier VARCHAR DEFAULT NULL
)
RETURNS INTEGER AS
$$
DECLARE
  current_category_id INTEGER;
  current_data_source_id INTEGER;
  current_qualifier_id INTEGER;
  new_category_id INTEGER;
  new_cluster_id VARCHAR;
  new_qualifier_id INTEGER;
BEGIN
  -- Find data_source_id
  SELECT
    id
  INTO
    current_data_source_id
  FROM
    data_source
  WHERE
    data_source.topic_name = data_source_name
  LIMIT 1;

  -- Stop now if SELECT returns no data_source_id
  IF current_data_source_id IS NULL THEN
    RETURN 0;
  END IF;

  -- Find current values of category_id, and qualifier_id
  SELECT
    category_id, qualifier_id
  INTO
    current_category_id, current_qualifier_id
  FROM cluster
  WHERE cluster.cluster_id = current_cluster
    and cluster.data_source_id = current_data_source_id
  LIMIT 1;

  -- Stop now if SELECT returns no cluster
  IF current_category_id IS NULL THEN
    RETURN 0;
  END IF;

  -- if new_category argument is null, use the value of 
  -- current_category_id as new_category_id
  IF new_category IS NULL THEN
    new_category_id := current_category_id;
  ELSE
    SELECT
      id
    INTO new_category_id
    FROM category
    WHERE category.name = new_category
    LIMIT 1;
    
    IF new_category_id is NULL THEN
      RETURN 0;
    END IF;
  END IF;

  -- if new_cluster argument is null, use the value of 
  -- current_cluster_id as new_cluster_id
  IF new_cluster IS NULL THEN
    new_cluster_id := current_cluster;
  ELSE
    new_cluster_id := new_cluster;
  END IF;

  -- if new_qualifier argument is null, use the value of 
  -- current_qualifier_id as new_qualifier_id
  IF new_qualifier IS NULL THEN
    new_qualifier_id := current_qualifier_id;
  ELSE
    SELECT
      id
    INTO new_qualifier_id
    FROM qualifier
    WHERE qualifier.description = new_qualifier 
    LIMIT 1;

    IF new_qualifier_id is NULL THEN
      RETURN 0;
    END IF;
  END IF;

  UPDATE cluster
  SET
    category_id = new_category_id,
    cluster_id = new_cluster_id,
    qualifier_id = new_qualifier_id,
    last_modification_time = CURRENT_TIMESTAMP(0)
  WHERE cluster.cluster_id = current_cluster
    and cluster.data_source_id = current_data_source_id;
  RETURN 1;
END;
$$ LANGUAGE plpgsql;

/******************************************************
 * ATTEMPT QUALIFIER_ID UPDATE
 *
 * attempt to update the value of qualifier_id 
 * in a specific cluster.
 * Return the number of rows updated (0 or 1)
 ******************************************************/
CREATE OR REPLACE FUNCTION attempt_qualifier_id_update(
  current_cluster VARCHAR,
  data_source_name VARCHAR,
  new_qualifier VARCHAR
)
RETURNS INTEGER AS
$$
DECLARE
  current_data_source_id INTEGER;
  new_qualifier_id INTEGER;
BEGIN
  -- Find id of the new qualifier from qualifier table
  SELECT
    id
  INTO new_qualifier_id
  FROM qualifier
  WHERE qualifier.description = new_qualifier 
  LIMIT 1;

  IF new_qualifier_id is NULL THEN
    RETURN 0;
  END IF;

  -- Find data_source_id
  SELECT
    id
  INTO
    current_data_source_id
  FROM
    data_source
  WHERE
    data_source.topic_name = data_source_name
  LIMIT 1;

  -- Stop now if SELECT returns no data_source_id
  IF current_data_source_id IS NULL THEN
    RETURN 0;
  END IF;

  UPDATE cluster
  SET
    qualifier_id = new_qualifier_id,
    last_modification_time = CURRENT_TIMESTAMP(0)
  WHERE cluster.cluster_id = current_cluster
    and cluster.data_source_id = current_data_source_id;
  RETURN 1;
END;
$$ LANGUAGE plpgsql;


/******************************************************
 * STATUS_ID UPDATE
 *
 * Update status_id to id of `reviewed` in status table
 * Called when qualifier_update_trigger is fired.
 ******************************************************/
CREATE OR REPLACE FUNCTION status_id_update()
RETURNS TRIGGER AS 
$$
DECLARE
  new_status_id INTEGER;
BEGIN
  SELECT
    id
  INTO new_status_id
  FROM status
  WHERE status.description = 'reviewed'
  LIMIT 1;

  IF new_status_id IS NOT NULL THEN
    UPDATE cluster
    SET status_id = new_status_id
    WHERE cluster.id = new.id;
  END IF;  
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

/******************************************************
* CREATE TRIGGERS
/******************************************************/
CREATE TRIGGER qualifier_update_trigger
  AFTER UPDATE ON cluster
  FOR EACH ROW
  WHEN (old.qualifier_id != new.qualifier_id)
  EXECUTE PROCEDURE status_id_update();
