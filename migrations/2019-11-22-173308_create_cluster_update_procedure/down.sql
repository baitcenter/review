DROP FUNCTION IF EXISTS attempt_cluster_update(VARCHAR, VARCHAR, VARCHAR, VARCHAR, VARCHAR);
DROP FUNCTION IF EXISTS attempt_qualifier_in_cluster_update(VARCHAR, VARCHAR, VARCHAR);
DROP TRIGGER IF EXISTS qualifier_update_trigger on cluster;
DROP FUNCTION IF EXISTS status_id_update();
