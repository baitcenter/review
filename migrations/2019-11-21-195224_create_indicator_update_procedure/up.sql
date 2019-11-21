/******************************************************
 * ATTEMPT INDICATOR UPDATE
 *
 * Attempt to update a specific indicator.
 * Return the number of rows updated (0 or 1)
 ******************************************************/
CREATE FUNCTION attempt_indicator_update(
  current_name VARCHAR,
  new_name VARCHAR DEFAULT NULL,
  new_token JSONB DEFAULT NULL,
  new_data_source VARCHAR DEFAULT NULL,
  new_description VARCHAR DEFAULT NULL
)
RETURNS INTEGER AS
$$
DECLARE
  current_token JSONB;
  current_data_source_id INTEGER;
  current_description VARCHAR;
  new_data_source_id INTEGER;
BEGIN
  -- Find current values of data_source_id and description
  SELECT
    token, data_source_id, description
  INTO
    current_token, current_data_source_id, current_description
  FROM indicator
  WHERE indicator.name = current_name
  LIMIT 1;

  -- Stop now if SELECT returns no indicator
  IF current_token IS NULL THEN
    RETURN 0;
  END IF;

  -- if new_token argument is null, use the value of 
  -- current_token as new_token
  IF new_token IS NULL THEN
    new_token := current_token;
  END IF;

  -- if new_data_source argument is null, use the value of 
  -- current_data_source_id as new_data_source_id
  IF new_data_source IS NULL THEN
    new_data_source_id := current_data_source_id;
  ELSE
    SELECT
      id
    INTO new_data_source_id
    FROM data_source
    WHERE data_source.topic_name = new_data_source
    LIMIT 1;
    
    IF new_data_source_id is NULL THEN
      RETURN 0;
    END IF;
  END IF;

  -- if new_name argument is null, use the value of 
  -- current_name as new_name
  IF new_name IS NULL THEN
    new_name := current_name;
  END IF;

  -- if new_description argument is null, use the value of 
  -- current_description as new_description
  IF new_description IS NULL THEN
    new_description := current_description;
  END IF;

  UPDATE indicator
  SET
    name = new_name,
    token = new_token,
    data_source_id = new_data_source_id,
    description = new_description,
    last_modification_time = CURRENT_TIMESTAMP(0)
  WHERE indicator.name = current_name;
  RETURN 1;

END;
$$ LANGUAGE plpgsql;
