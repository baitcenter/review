#![allow(non_snake_case)]

table! {
    Action (action_id) {
        action_id -> Nullable<Integer>,
        action -> Text,
    }
}

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
        description -> Nullable<Text>,
        category_id -> Integer,
        detector_id -> Integer,
        examples -> Nullable<Binary>,
        priority_id -> Integer,
        qualifier_id -> Integer,
        status_id -> Integer,
        rules -> Nullable<Text>,
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
        size -> Nullable<Text>,
    }
}

table! {
    Priority (priority_id) {
        priority_id -> Nullable<Integer>,
        priority -> Text,
    }
}

table! {
    Qualifier (qualifier_id) {
        qualifier_id -> Nullable<Integer>,
        qualifier -> Text,
    }
}

table! {
    RawEvent (event_id) {
        event_id -> Text,
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
joinable!(Clusters -> Priority (priority_id));
joinable!(Clusters -> Qualifier (qualifier_id));
joinable!(Clusters -> Status (status_id));
joinable!(Clusters -> DataSource (data_source_id));
joinable!(Outliers -> DataSource (data_source_id));
joinable!(RawEvent -> DataSource (data_source_id));

allow_tables_to_appear_in_same_query!(
    Action, Category, Clusters, DataSource, Outliers, Priority, Qualifier, RawEvent, Status,
);
