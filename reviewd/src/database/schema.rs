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
        event_ids -> Nullable<Array<Numeric>>,
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
    column_description (id) {
        id -> Int4,
        cluster_id -> Int4,
        first_event_id -> Text,
        last_event_id -> Text,
        column_index -> Int4,
        type_id -> Int4,
        count -> Int8,
        unique_count -> Int8,
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
    description_datetime (id) {
        id -> Int4,
        description_id -> Int4,
        mode -> Nullable<Timestamp>,
    }
}

table! {
    description_element_type (id) {
        id -> Int4,
        name -> Text,
    }
}

table! {
    description_enum (id) {
        id -> Int4,
        description_id -> Int4,
        mode -> Nullable<Int8>,
    }
}

table! {
    description_float (id) {
        id -> Int4,
        description_id -> Int4,
        min -> Nullable<Float8>,
        max -> Nullable<Float8>,
        mean -> Nullable<Float8>,
        s_deviation -> Nullable<Float8>,
        mode_smallest -> Nullable<Float8>,
        mode_largest -> Nullable<Float8>,
    }
}

table! {
    description_int (id) {
        id -> Int4,
        description_id -> Int4,
        min -> Nullable<Int8>,
        max -> Nullable<Int8>,
        mean -> Nullable<Float8>,
        s_deviation -> Nullable<Float8>,
        mode -> Nullable<Int8>,
    }
}

table! {
    description_ipaddr (id) {
        id -> Int4,
        description_id -> Int4,
        mode -> Nullable<Text>,
    }
}

table! {
    description_text (id) {
        id -> Int4,
        description_id -> Int4,
        mode -> Nullable<Text>,
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
        raw_event -> Text,
        data_source_id -> Int4,
        event_ids -> Array<Numeric>,
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
        data -> Text,
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

table! {
    top_n_datetime (id) {
        id -> Int4,
        description_id -> Int4,
        ranking -> Nullable<Int8>,
        value -> Nullable<Timestamp>,
        count -> Nullable<Int8>,
    }
}

table! {
    top_n_enum (id) {
        id -> Int4,
        description_id -> Int4,
        ranking -> Nullable<Int8>,
        value -> Nullable<Int8>,
        count -> Nullable<Int8>,
    }
}

table! {
    top_n_float (id) {
        id -> Int4,
        description_id -> Int4,
        ranking -> Nullable<Int8>,
        value_smallest -> Nullable<Float8>,
        value_largest -> Nullable<Float8>,
        count -> Nullable<Int8>,
    }
}

table! {
    top_n_int (id) {
        id -> Int4,
        description_id -> Int4,
        ranking -> Nullable<Int8>,
        value -> Nullable<Int8>,
        count -> Nullable<Int8>,
    }
}

table! {
    top_n_ipaddr (id) {
        id -> Int4,
        description_id -> Int4,
        ranking -> Nullable<Int8>,
        value -> Nullable<Text>,
        count -> Nullable<Int8>,
    }
}

table! {
    top_n_text (id) {
        id -> Int4,
        description_id -> Int4,
        ranking -> Nullable<Int8>,
        value -> Nullable<Text>,
        count -> Nullable<Int8>,
    }
}

joinable!(cluster -> category (category_id));
joinable!(cluster -> data_source (data_source_id));
joinable!(cluster -> qualifier (qualifier_id));
joinable!(cluster -> raw_event (raw_event_id));
joinable!(cluster -> status (status_id));
joinable!(column_description -> cluster (cluster_id));
joinable!(column_description -> description_element_type (type_id));
joinable!(description_datetime -> column_description (description_id));
joinable!(description_enum -> column_description (description_id));
joinable!(description_float -> column_description (description_id));
joinable!(description_int -> column_description (description_id));
joinable!(description_ipaddr -> column_description (description_id));
joinable!(description_text -> column_description (description_id));
joinable!(indicator -> category (category));
joinable!(indicator -> data_source (source));
joinable!(outlier -> data_source (data_source_id));
joinable!(outlier -> raw_event (raw_event_id));
joinable!(raw_event -> data_source (data_source_id));
joinable!(token -> indicator (indicator));
joinable!(top_n_datetime -> column_description (description_id));
joinable!(top_n_enum -> column_description (description_id));
joinable!(top_n_float -> column_description (description_id));
joinable!(top_n_int -> column_description (description_id));
joinable!(top_n_ipaddr -> column_description (description_id));
joinable!(top_n_text -> column_description (description_id));

allow_tables_to_appear_in_same_query!(
    category,
    cluster,
    column_description,
    data_source,
    description_datetime,
    description_element_type,
    description_enum,
    description_float,
    description_int,
    description_ipaddr,
    description_text,
    indicator,
    outlier,
    qualifier,
    raw_event,
    status,
    token,
    top_n_datetime,
    top_n_enum,
    top_n_float,
    top_n_int,
    top_n_ipaddr,
    top_n_text,
);
