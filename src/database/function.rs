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
    fn attempt_indicator_update (
        indicator_name: Varchar,
        new_indicator_name: Nullable<Varchar>,
        new_token: Nullable<Jsonb>,
        new_data_source: Nullable<Varchar>,
        new_description: Nullable<Varchar>
    ) -> Integer;
}

sql_function! {
    fn attempt_qualifier_id_update (
            cluster_id: Varchar,
            data_source: Varchar,
            new_qualifier: Varchar
        ) -> Integer;
}
