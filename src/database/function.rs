use diesel::sql_types::*;

sql_function! {
    fn attempt_cluster_update (
        cluster_id: Varchar,
        data_source: Varchar,
        new_category: Nullable<Varchar>,
        new_cluster_id: Nullable<Varchar>,
        new_qualifier: Nullable<Varchar>
    ) -> Integer;
}

sql_function! {
    #[allow(clippy::too_many_arguments)]
    fn attempt_cluster_upsert (
        max_event_id_num: Numeric,
        cluster_id: Varchar,
        topic_name: Varchar,
        data_type: Varchar,
        detector_id: Integer,
        event_ids: Nullable<Array<Numeric>>,
        signature: Nullable<Varchar>,
        score: Nullable<Float8>,
        size: Nullable<Numeric>
    ) -> Integer;
}

sql_function! {
    fn attempt_event_ids_update (
        max_event_id_num: Numeric
    );
}

sql_function! {
    fn attempt_indicator_update (
        indicator_name: Varchar,
        new_indicator_name: Nullable<Varchar>,
        new_token: Nullable<Jsonb>,
        new_data_source: Nullable<Varchar>,
        new_description: Nullable<Varchar>
    ) -> Integer;
}

sql_function! {
    fn attempt_outlier_upsert (
        max_event_id_num: Numeric,
        id: Integer,
        raw_event: Bytea,
        topic_name: Varchar,
        data_type: Varchar,
        event_ids: Array<Numeric>,
        size: Numeric
    ) -> Integer;
}

sql_function! {
    fn attempt_qualifier_id_update (
        cluster_id: Varchar,
        data_source: Varchar,
        new_qualifier: Varchar
    ) -> Integer;
}

sql_function! {
    fn lookup_events_with_no_raw_event (
        data_source_id: Integer
    ) -> Numeric;
}

sql_function! {
    fn lookup_kafka_metadata (
        data_source_id: Integer,
        messae_id: Numeric
    ) -> Nullable<Jsonb>;
}
