use actix_web::{
    error, guard,
    web::{delete, get, post, put, resource, Json, PathConfig, Query, ServiceConfig},
    FromRequest, HttpResponse,
};
use serde_json::Value;

use crate::database::*;

#[allow(clippy::too_many_lines)]
pub(crate) fn init_app(cfg: &mut ServiceConfig) {
    cfg.service(
        resource("/api/category")
            .guard(guard::Any(guard::Get()).or(guard::Post()))
            .route(get().to(get_category_table))
            .data(Query::<NewCategory>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(post().to(add_category)),
    )
    .service(
        resource("/api/category/{category}")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .data(PathConfig::default().error_handler(|err, _| {
                error::InternalError::from_response(err, HttpResponse::BadRequest().finish()).into()
            }))
            .data(Json::<NewCategory>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to(update_category)),
    )
    .service(
        resource("/api/cluster")
            .guard(guard::Get())
            .route(get().to(get_clusters)),
    )
    .service(
        resource("/api/cluster")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .route(put().to(update_clusters)),
    )
    .service(
        resource("/api/cluster/qualifier")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .route(put().to(update_qualifiers)),
    )
    .service(
        resource("/api/cluster/{cluster_id}")
            .guard(guard::Header("content-type", "application/json"))
            .data(PathConfig::default().error_handler(|err, _| {
                error::InternalError::from_response(err, HttpResponse::BadRequest().finish()).into()
            }))
            .data(Query::<Value>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .data(Json::<Value>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to(update_cluster)),
    )
    .service(
        resource("/api/data_source")
            .guard(guard::Any(guard::Get()).or(guard::Post()))
            .route(get().to(get_data_source_table))
            .route(post().to(add_data_source)),
    )
    .service(
        resource("/api/description")
            .guard(guard::Get())
            .route(get().to(get_description)),
    )
    .service(
        resource("/api/description")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .route(put().to(add_descriptions)),
    )
    .service(
        resource("/api/description/round")
            .guard(guard::Get())
            .data(Query::<RoundSelectQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(get().to(get_rounds_by_cluster)),
    )
    .service(
        resource("/api/event")
            .guard(guard::Put())
            .route(put().to(update_events)),
    )
    .service(
        resource("/api/event/no_raw_events")
            .guard(guard::Get())
            .data(Query::<Value>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(get().to(get_events_with_no_raw_event)),
    )
    .service(
        resource("/api/event/search")
            .guard(guard::Get())
            .route(get().to(get_events)),
    )
    .service(
        resource("/api/event_id")
            .guard(guard::Get())
            .route(get().to(get_max_event_id_num)),
    )
    .service(
        resource("/api/event_id")
            .guard(guard::Put())
            .route(put().to(update_max_event_id_num)),
    )
    .service(
        resource("/api/indicator")
            .guard(guard::Post())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Value>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(post().to(add_indicator)),
    )
    .service(
        resource("/api/indicator")
            .guard(guard::Get())
            .route(get().to(get_indicators)),
    )
    .service(
        resource("/api/indicator")
            .guard(guard::Delete())
            .data(Query::<Value>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(delete().to(delete_indicator)),
    )
    .service(
        resource("/api/indicator/{name}")
            .guard(guard::Header("content-type", "application/json"))
            .data(PathConfig::default().error_handler(|err, _| {
                error::InternalError::from_response(err, HttpResponse::BadRequest().finish()).into()
            }))
            .data(Json::<Value>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to(update_indicator)),
    )
    .service(
        resource("/api/kafka_metadata")
            .guard(guard::Get())
            .route(get().to(get_kafka_metadata)),
    )
    .service(
        resource("/api/kafka_metadata")
            .guard(guard::Put())
            .data(Json::<Vec<KafkaMetadata>>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to(add_kafka_metadata)),
    )
    .service(
        resource("/api/outlier")
            .guard(guard::Get())
            .route(get().to(get_outliers)),
    )
    .service(
        resource("/api/outlier")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .route(put().to(update_outliers)),
    )
    .service(
        resource("/api/outlier")
            .guard(guard::Delete())
            .guard(guard::Header("content-type", "application/json"))
            .data(Query::<DataSourceQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(delete().to(delete_outliers)),
    )
    .service(
        resource("/api/qualifier")
            .guard(guard::Get())
            .route(get().to(get_qualifier_table)),
    )
    .service(
        resource("/api/status")
            .guard(guard::Get())
            .route(get().to(get_status_table)),
    )
    .service(
        resource("/api/template")
            .guard(guard::Get())
            .route(get().to(get_template)),
    )
    .service(
        resource("/api/template")
            .guard(guard::Post())
            .guard(guard::Header("content-type", "application/json"))
            .route(post().to(add_template)),
    )
    .service(
        resource("/api/template")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .route(put().to(update_template)),
    )
    .service(
        resource("/api/template")
            .guard(guard::Delete())
            .route(delete().to(delete_template)),
    )
    .service(
        resource("/api/experimental/cluster/total")
            .guard(guard::Get())
            .route(get().to(get_sum_of_cluster_sizes)),
    )
    .service(
        resource("/api/experimental/cluster/time_series")
            .guard(guard::Get())
            .route(get().to(get_cluster_time_series)),
    )
    .service(
        resource("/api/experimental/data_source/time_series/regression")
            .guard(guard::Get())
            .route(get().to(get_top_n_of_cluster_time_series_by_linear_regression)),
    )
    .service(
        resource("/api/experimental/cluster/top_n_ipaddr")
            .guard(guard::Get())
            .route(get().to(get_top_n_ipaddr_of_cluster)),
    );
}
