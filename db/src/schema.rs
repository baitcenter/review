#![allow(non_snake_case)]

table! {
    Category (category_id) {
        category_id -> Nullable<Integer>,
        category -> Text,
    }
}

table! {
    Clusters (id) {
        id -> Nullable<Integer>,
        cluster_id -> Nullable<Text>,
        category_id -> Integer,
        detector_id -> Integer,
        event_ids -> Nullable<Binary>,
        raw_event_id -> Nullable<Integer>,
        qualifier_id -> Integer,
        status_id -> Integer,
        signature -> Text,
        size -> Text,
        score -> Nullable<Double>,
        data_source_id -> Integer,
        last_modification_time -> Nullable<Timestamp>,
    }
}

table! {
    DataSource (data_source_id) {
        data_source_id -> Integer,
        topic_name -> Text,
        data_type -> Text,
    }
}

table! {
    Outliers (id) {
        id -> Nullable<Integer>,
        raw_event -> Binary,
        data_source_id -> Integer,
        event_ids -> Nullable<Binary>,
        raw_event_id -> Nullable<Integer>,
        size -> Nullable<Text>,
    }
}

table! {
    Qualifier (qualifier_id) {
        qualifier_id -> Nullable<Integer>,
        qualifier -> Text,
    }
}

table! {
    RawEvent (raw_event_id) {
        raw_event_id -> Integer,
        raw_event -> Binary,
        data_source_id -> Integer,
    }
}

table! {
    Status (status_id) {
        status_id -> Nullable<Integer>,
        status -> Text,
    }
}

joinable!(Clusters -> Category (category_id));
joinable!(Clusters -> Qualifier (qualifier_id));
joinable!(Clusters -> RawEvent (raw_event_id));
joinable!(Clusters -> Status (status_id));
joinable!(Clusters -> DataSource (data_source_id));
joinable!(Outliers -> DataSource (data_source_id));
joinable!(RawEvent -> DataSource (data_source_id));

allow_tables_to_appear_in_same_query!(
    Category, Clusters, DataSource, Outliers, Qualifier, RawEvent, Status,
);
