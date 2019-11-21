use diesel::sql_types::*;

sql_function! {
    fn attempt_indicator_update (
        indicator_name: Varchar,
        new_indicator_name: Nullable<Varchar>,
        new_token: Nullable<Jsonb>,
        new_data_source: Nullable<Varchar>,
        new_description: Nullable<Varchar>
    ) -> Integer;
}
