table! {
    category (id) {
        id -> Int4,
        name -> Text,
    }
}

table! {
    cluster (id) {
        id -> Int4,
        cluster_id -> Nullable<Text>,
        category_id -> Int4,
        detector_id -> Int4,
        event_ids -> Nullable<Bytea>,
        raw_event_id -> Nullable<Int4>,
        qualifier_id -> Int4,
        status_id -> Int4,
        signature -> Text,
        size -> Text,
        score -> Nullable<Float8>,
        data_source_id -> Int4,
        last_modification_time -> Nullable<Timestamp>,
    }
}

table! {
    data_source (id) {
        id -> Int4,
        topic_name -> Text,
        data_type -> Text,
    }
}

table! {
    indicator (id) {
        id -> Int4,
        description -> Nullable<Text>,
        source -> Nullable<Int4>,
        category -> Nullable<Int4>,
        qualification -> Nullable<Float8>,
    }
}

table! {
    outlier (id) {
        id -> Int4,
        raw_event -> Bytea,
        data_source_id -> Int4,
        event_ids -> Bytea,
        raw_event_id -> Nullable<Int4>,
        size -> Nullable<Text>,
    }
}

table! {
    qualifier (id) {
        id -> Int4,
        description -> Text,
    }
}

table! {
    raw_event (id) {
        id -> Int4,
        data -> Bytea,
        data_source_id -> Int4,
    }
}

table! {
    status (id) {
        id -> Int4,
        description -> Text,
    }
}

table! {
    token (id) {
        id -> Int4,
        name -> Nullable<Bytea>,
        indicator -> Nullable<Int4>,
    }
}

joinable!(cluster -> category (category_id));
joinable!(cluster -> data_source (data_source_id));
joinable!(cluster -> qualifier (qualifier_id));
joinable!(cluster -> raw_event (raw_event_id));
joinable!(cluster -> status (status_id));
joinable!(indicator -> category (category));
joinable!(indicator -> data_source (source));
joinable!(outlier -> data_source (data_source_id));
joinable!(outlier -> raw_event (raw_event_id));
joinable!(raw_event -> data_source (data_source_id));
joinable!(token -> indicator (indicator));

allow_tables_to_appear_in_same_query!(
    category,
    cluster,
    data_source,
    indicator,
    outlier,
    qualifier,
    raw_event,
    status,
    token,
);
