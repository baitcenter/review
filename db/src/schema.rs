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
    Detectors (detector_id) {
        detector_id -> Nullable<Integer>,
        detector_type -> Integer,
        description -> Nullable<Text>,
        input -> Nullable<Text>,
        local_db -> Nullable<Text>,
        output -> Text,
        status_id -> Integer,
        suspicious_tokens -> Nullable<Text>,
        target_detector_id -> Nullable<Integer>,
        time_regex -> Nullable<Text>,
        time_format -> Nullable<Text>,
        last_modification_time -> Nullable<Timestamp>,
    }
}

table! {
    Events (event_id) {
        event_id -> Nullable<Integer>,
        cluster_id -> Nullable<Text>,
        description -> Nullable<Text>,
        category_id -> Integer,
        detector_id -> Integer,
        examples -> Nullable<Text>,
        priority_id -> Integer,
        qualifier_id -> Integer,
        status_id -> Integer,
        rules -> Nullable<Text>,
        signature -> Text,
        last_modification_time -> Nullable<Timestamp>,
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
    Ready_table (publish_id) {
        publish_id -> Nullable<Integer>,
        action_id -> Integer,
        event_id -> Nullable<Integer>,
        detector_id -> Nullable<Integer>,
        time_published -> Nullable<Timestamp>,
    }
}

table! {
    Status (status_id) {
        status_id -> Nullable<Integer>,
        status -> Text,
    }
}

joinable!(Detectors -> Status (status_id));
joinable!(Events -> Category (category_id));
joinable!(Events -> Detectors (detector_id));
joinable!(Events -> Priority (priority_id));
joinable!(Events -> Qualifier (qualifier_id));
joinable!(Events -> Status (status_id));
joinable!(Ready_table -> Action (action_id));
joinable!(Ready_table -> Detectors (detector_id));
joinable!(Ready_table -> Events (event_id));

allow_tables_to_appear_in_same_query!(
    Action,
    Category,
    Detectors,
    Events,
    Priority,
    Qualifier,
    Ready_table,
    Status,
);
