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
        qualifier_id -> Int4,
        status_id -> Int4,
        signature -> Text,
        size -> Numeric,
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
    description_binary (id) {
        id -> Int4,
        description_id -> Int4,
        mode -> Nullable<Bytea>,
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
        mode -> Nullable<Text>,
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
    event (id) {
        id -> Int4,
        message_id -> Numeric,
        data_source_id -> Int4,
        raw_event -> Nullable<Bytea>,
    }
}

table! {
    indicator (id) {
        id -> Int4,
        name -> Text,
        description -> Nullable<Text>,
        token -> Jsonb,
        data_source_id -> Int4,
        last_modification_time -> Nullable<Timestamp>,
    }
}

table! {
    kafka_metadata (id) {
        id -> Int4,
        data_source_id -> Int4,
        partition -> Int4,
        offsets -> Int8,
        message_ids -> Numrange,
    }
}

table! {
    outlier (id) {
        id -> Int4,
        raw_event -> Bytea,
        data_source_id -> Int4,
        event_ids -> Array<Numeric>,
        size -> Numeric,
    }
}

table! {
    qualifier (id) {
        id -> Int4,
        description -> Text,
    }
}

table! {
    status (id) {
        id -> Int4,
        description -> Text,
    }
}

table! {
    template (id) {
        id -> Int4,
        name -> Text,
        event_type -> Text,
        method -> Text,
        algorithm -> Nullable<Text>,
        min_token_length -> Nullable<Int8>,
        eps -> Nullable<Float8>,
        format -> Nullable<Jsonb>,
        dimension_default -> Nullable<Int8>,
        dimensions -> Nullable<Array<Int8>>,
        time_intervals -> Nullable<Array<Text>>,
        numbers_of_top_n -> Nullable<Array<Int8>>,
    }
}

table! {
    top_n_binary (id) {
        id -> Int4,
        description_id -> Int4,
        ranking -> Nullable<Int8>,
        value -> Nullable<Bytea>,
        count -> Nullable<Int8>,
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
        value -> Nullable<Text>,
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

joinable!(column_description -> cluster (cluster_id));
joinable!(column_description -> description_element_type (type_id));
joinable!(description_binary -> column_description (description_id));
joinable!(description_datetime -> column_description (description_id));
joinable!(description_enum -> column_description (description_id));
joinable!(description_float -> column_description (description_id));
joinable!(description_int -> column_description (description_id));
joinable!(description_ipaddr -> column_description (description_id));
joinable!(description_text -> column_description (description_id));
joinable!(top_n_binary -> column_description (description_id));
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
    description_binary,
    description_datetime,
    description_element_type,
    description_enum,
    description_float,
    description_int,
    description_ipaddr,
    description_text,
    event,
    indicator,
    kafka_metadata,
    outlier,
    qualifier,
    status,
    template,
    top_n_binary,
    top_n_datetime,
    top_n_enum,
    top_n_float,
    top_n_int,
    top_n_ipaddr,
    top_n_text,
);
