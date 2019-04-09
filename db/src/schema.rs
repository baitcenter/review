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
    Events (event_id) {
        event_id -> Nullable<Integer>,
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
    Status (status_id) {
        status_id -> Nullable<Integer>,
        status -> Text,
    }
}

joinable!(Events -> Category (category_id));
joinable!(Events -> Priority (priority_id));
joinable!(Events -> Qualifier (qualifier_id));
joinable!(Events -> Status (status_id));

allow_tables_to_appear_in_same_query!(Action, Category, Events, Priority, Qualifier, Status,);
