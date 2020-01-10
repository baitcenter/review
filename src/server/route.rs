use actix_web::{
    error, guard, http,
    web::{
        delete, get, post, put, resource, BytesMut, Data, Json, PathConfig, Payload, Query,
        ServiceConfig,
    },
    FromRequest, HttpResponse,
};
use futures::stream::StreamExt;
use serde::Deserialize;
use serde_json::Value;

use crate::database::*;
use crate::server::EtcdServer;

#[derive(Debug, Deserialize)]
struct EtcdKeyQuery {
    etcd_key: String,
}

async fn send_suspicious_tokens_to_etcd(
    mut payload: Payload,
    etcd_key: Query<EtcdKeyQuery>,
    etcd_server: Data<EtcdServer>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut body = BytesMut::new();
    while let Some(item) = payload.next().await {
        body.extend_from_slice(&item?);
    }

    let data = format!(
        r#"{{"key": "{}", "value": "{}"}}"#,
        base64::encode(&etcd_key.into_inner().etcd_key),
        base64::encode(&body)
    );

    let response = reqwest::Client::new()
        .post(&etcd_server.into_inner().etcd_url)
        .body(data)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status);
    if let Err(e) = response {
        Ok(HttpResponse::InternalServerError()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(build_err_msg(&e)))
    } else {
        Ok(HttpResponse::Ok().finish())
    }
}

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
            .route(post().to(add_data_source_endpoint)),
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
            .data(Json::<Vec<DescriptionInsert>>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
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
        resource("/api/etcd/suspicious_tokens")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .data(Query::<EtcdKeyQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to(send_suspicious_tokens_to_etcd)),
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
            .data(Json::<Vec<GetEvent>>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(get().to(get_events)),
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
    );
}
