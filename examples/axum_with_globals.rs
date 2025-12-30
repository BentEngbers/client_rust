use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client_derive_encode::{EncodeLabelSet, EncodeLabelValue};
use std::sync::LazyLock;

//A separate module (usually in a seperate file) to scope the registry
mod metrics {
    use axum::body::Body;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use prometheus_client::encoding::text::encode;
    use prometheus_client::registry::{Metric, Registry};
    use std::sync::{LazyLock, Mutex};

    static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| Mutex::new(Registry::default()));

    pub fn register_metric_to_global_registry<MetricType: Metric + Clone + Default>(
        name: &str,
        help: &str,
    ) -> MetricType {
        let metric: MetricType = MetricType::default();
        let mut registry = REGISTRY.lock().expect(&format!(
            "Cannot lock metrics registry to create {name} metric"
        ));
        registry.register(name, help, metric.clone());
        metric
    }

    pub async fn metrics_handler() -> impl IntoResponse {
        let mut buffer = String::new();
        {
            let registry = REGISTRY
                .lock()
                .expect("could not acquire a lock on registry to push metrics");
            encode(&mut buffer, &registry).unwrap();
        }

        Response::builder()
            .status(StatusCode::OK)
            .header(
                CONTENT_TYPE,
                "application/openmetrics-text; version=1.0.0; charset=utf-8",
            )
            .body(Body::from(buffer))
            .unwrap()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum Method {
    Get,
    Post,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct MethodLabels {
    pub method: Method,
}

static METRIC: LazyLock<Family<MethodLabels, Counter>> =
    LazyLock::new(|| metrics::register_metric_to_global_registry("requests", "Count of requests"));

pub async fn some_handler() -> impl IntoResponse {
    METRIC
        .get_or_create(&MethodLabels {
            method: Method::Get,
        })
        .inc();
    "okay".to_string()
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .route("/metrics", get(metrics::metrics_handler))
        .route("/handler", get(some_handler));
    let port = 8080;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    axum::serve(listener, router).await.unwrap();
}
