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
        data_source -> Text,
        last_modification_time -> Nullable<Timestamp>,
    }
}

table! {
    Outliers (outlier_id) {
        outlier_id -> Nullable<Integer>,
        outlier_raw_event -> Binary,
        outlier_data_source -> Text,
        outlier_event_ids -> Nullable<Binary>,
        outlier_size -> Nullable<Text>,
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
        data_source -> Text,
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

allow_tables_to_appear_in_same_query!(
    Action, Category, Clusters, Outliers, Priority, Qualifier, RawEvent, Status,
);
