use actix_web::{
    error, guard, http,
    web::{
        delete, get, post, put, resource, BytesMut, Data, Json, PathConfig, Payload, Query,
        ServiceConfig,
    },
    FromRequest, HttpResponse,
};
use futures::{Future, Stream};
use serde::Deserialize;
use serde_json::Value;

use crate::database::*;
use crate::server::EtcdServer;

#[derive(Debug, Deserialize)]
struct EtcdKeyQuery {
    etcd_key: String,
}

fn send_suspicious_tokens_to_etcd(
    body: Payload,
    etcd_key: Query<EtcdKeyQuery>,
    etcd_server: Data<EtcdServer>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    body.map_err(actix_web::Error::from)
        .fold(BytesMut::new(), move |mut body, chunk| {
            body.extend_from_slice(&chunk);
            Ok::<_, actix_web::Error>(body)
        })
        .and_then(|body| {
            let data = format!(
                r#"{{"key": "{}", "value": "{}"}}"#,
                base64::encode(&etcd_key.into_inner().etcd_key),
                base64::encode(&body)
            );

            let (tx, rx) = tokio::sync::oneshot::channel();
            let req = reqwest::r#async::Client::new()
                .post(&etcd_server.into_inner().etcd_url)
                .body(data)
                .send()
                .and_then(reqwest::r#async::Response::error_for_status)
                .then(move |response| tx.send(response))
                .map(|_| ())
                .map_err(|_| ());
            if let Ok(mut runtime) = tokio::runtime::Runtime::new() {
                runtime.spawn(req);
                let response = rx.wait();
                if let Ok(Ok(_)) = response {
                    return Ok(HttpResponse::Ok().finish());
                } else if let Ok(Err(e)) = response {
                    return Ok(HttpResponse::InternalServerError()
                        .header(http::header::CONTENT_TYPE, "application/json")
                        .body(build_err_msg(&e)));
                }
            }

            Ok(HttpResponse::InternalServerError()
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(r#"{{"message": "Failed to update etcd"}}"#))
        })
}

#[allow(clippy::too_many_lines)]
pub(crate) fn init_app(cfg: &mut ServiceConfig) {
    cfg.service(
        resource("/api/category")
            .guard(guard::Any(guard::Get()).or(guard::Post()))
            .route(get().to_async(get_category_table))
            .data(Query::<NewCategory>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(post().to_async(add_category)),
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
            .route(put().to_async(update_category)),
    )
    .service(
        resource("/api/cluster")
            .guard(guard::Get())
            .route(get().to_async(get_clusters)),
    )
    .service(
        resource("/api/cluster")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<ClusterUpdate>>::configure(|cfg| {
                // increase max size of payload from 32kb to 1024kb
                cfg.limit(1_048_576).error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to_async(update_clusters)),
    )
    .service(
        resource("/api/cluster/qualifier")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<QualifierUpdate>>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to_async(update_qualifiers)),
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
            .route(put().to_async(update_cluster)),
    )
    .service(
        resource("/api/data_source")
            .guard(guard::Any(guard::Get()).or(guard::Post()))
            .route(get().to_async(get_data_source_table))
            .route(post().to_async(add_data_source_endpoint)),
    )
    .service(
        resource("/api/description")
            .guard(guard::Get())
            .route(get().to_async(get_description)),
    )
    .service(
        resource("/api/description")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<DescriptionUpdate>>::configure(|cfg| {
                // increase max size of payload from 32kb to 1024kb
                cfg.limit(1_048_576).error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to_async(add_descriptions)),
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
            .route(get().to_async(get_rounds_by_cluster)),
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
            .route(put().to_async(send_suspicious_tokens_to_etcd)),
    )
    .service(
        resource("/api/indicator")
            .guard(guard::Post())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Value>::configure(|cfg| {
                // increase max size of payload from 32kb to 1024kb
                cfg.limit(1_048_576).error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(post().to_async(add_indicator)),
    )
    .service(
        resource("/api/indicator")
            .guard(guard::Get())
            .route(get().to_async(get_indicators)),
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
            .route(delete().to_async(delete_indicator)),
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
            .route(put().to_async(update_indicator)),
    )
    .service(
        resource("/api/outlier")
            .guard(guard::Get())
            .route(get().to_async(get_outliers)),
    )
    .service(
        resource("/api/outlier")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<OutlierUpdate>>::configure(|cfg| {
                // increase max size of payload from 32kb to 1024kb
                cfg.limit(1_048_576).error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to_async(update_outliers)),
    )
    .service(
        resource("/api/outlier")
            .guard(guard::Delete())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<String>>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .data(Query::<DataSourceQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(delete().to_async(delete_outliers)),
    )
    .service(
        resource("/api/qualifier")
            .guard(guard::Get())
            .route(get().to_async(get_qualifier_table)),
    )
    .service(
        resource("/api/raw_events")
            .guard(guard::Put())
            .data(Query::<RawEventQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to_async(add_raw_events)),
    )
    .service(
        resource("/api/status")
            .guard(guard::Get())
            .route(get().to_async(get_status_table)),
    );
}