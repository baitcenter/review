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
                .and_then(|response| response.error_for_status())
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
            .guard(guard::Any(guard::Get()).or(guard::Post()).or(guard::Put()))
            .route(get().to_async(get_cluster_table))
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<ClusterUpdate>>::configure(|cfg| {
                // increase max size of payload from 32kb to 1024kb
                cfg.limit(1_048_576).error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(post().to_async(add_clusters))
            .route(put().to_async(update_clusters)),
    )
    .service(
        resource("/api/cluster/{cluster_id}")
            .guard(guard::Header("content-type", "application/json"))
            .data(PathConfig::default().error_handler(|err, _| {
                error::InternalError::from_response(err, HttpResponse::BadRequest().finish()).into()
            }))
            .data(Query::<DataSourceQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .data(Json::<Vec<NewClusterValues>>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to_async(update_cluster)),
    )
    .service(
        resource("/api/cluster/qualifier")
            .guard(guard::Put())
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<ClusterSelectQuery>>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(put().to_async(update_qualifiers)),
    )
    .service(
        resource("/api/cluster/search")
            .guard(guard::Get())
            .data(Query::<DataSourceQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(get().to_async(get_selected_clusters)),
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
        resource("/api/outlier")
            .guard(guard::Any(guard::Get()).or(guard::Post()).or(guard::Put()))
            .route(get().to_async(get_outlier_table))
            .guard(guard::Header("content-type", "application/json"))
            .data(Json::<Vec<OutlierUpdate>>::configure(|cfg| {
                // increase max size of payload from 32kb to 1024kb
                cfg.limit(1_048_576).error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(post().to_async(add_outliers))
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
        resource("/api/outlier/{data_source}")
            .guard(guard::Get())
            .data(Query::<DataSourceQuery>::configure(|cfg| {
                cfg.error_handler(|err, _| {
                    error::InternalError::from_response(err, HttpResponse::BadRequest().finish())
                        .into()
                })
            }))
            .route(get().to_async(get_outlier_by_data_source)),
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
