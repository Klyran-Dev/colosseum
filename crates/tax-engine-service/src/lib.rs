#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use arc_swap::ArcSwap;
use axum::{
    Router,
    extract::{
        Json, Request, State,
        rejection::{BytesRejection, FailedToBufferBody, JsonRejection},
    },
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use observability::ServiceMetrics;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tax_engine::{
    CalculationResult, CompiledRuleSet, RuleSetDocument, TaxDocumentFacts, TaxDocumentResult,
    TaxFacts,
};
use thiserror::Error;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tower_http::limit::RequestBodyLimitLayer;

pub const DEFAULT_CACHE_CAPACITY: usize = 64;
pub const DEFAULT_MAX_CONCURRENCY: usize = 32;
pub const MAX_CACHE_CAPACITY: usize = 256;
pub const MAX_CONCURRENCY: usize = 256;
pub const MAX_REQUEST_BODY_BYTES: usize = 128 * 1024;
pub const MAX_QUEUE_WAIT: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CalculateRequest {
    pub rule_set: RuleSetDocument,
    pub facts: TaxFacts,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CalculateDocumentRequest {
    pub rule_set: RuleSetDocument,
    pub facts: TaxDocumentFacts,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("service authentication failed")]
    Unauthorized,
    #[error("invalid tax calculation request: {0}")]
    InvalidRequest(String),
    #[error("tax calculation request body exceeds the maximum size")]
    PayloadTooLarge,
    #[error("tax engine service is overloaded")]
    Overloaded,
    #[error("tax engine worker failed")]
    WorkerFailed,
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Overloaded | Self::WorkerFailed => StatusCode::SERVICE_UNAVAILABLE,
        };
        (
            status,
            axum::Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}

struct Cache {
    capacity: usize,
    entries: HashMap<[u8; 32], Arc<CompiledRuleSet>>,
    order: VecDeque<[u8; 32]>,
}

struct EngineInner {
    cache: Mutex<Cache>,
    concurrency: Arc<Semaphore>,
    metrics: ServiceMetrics,
    cache_capacity: usize,
}

#[derive(Clone)]
pub struct EngineService {
    inner: Arc<EngineInner>,
}

impl EngineService {
    pub fn new(cache_capacity: usize) -> Result<Self, String> {
        Self::with_limits(cache_capacity, DEFAULT_MAX_CONCURRENCY)
    }

    pub fn with_limits(cache_capacity: usize, max_concurrency: usize) -> Result<Self, String> {
        if !(1..=MAX_CACHE_CAPACITY).contains(&cache_capacity) {
            return Err(format!(
                "tax engine cache capacity must be between 1 and {MAX_CACHE_CAPACITY}"
            ));
        }
        if !(1..=MAX_CONCURRENCY).contains(&max_concurrency) {
            return Err(format!(
                "tax engine concurrency must be between 1 and {MAX_CONCURRENCY}"
            ));
        }
        Ok(Self {
            inner: Arc::new(EngineInner {
                cache: Mutex::new(Cache {
                    capacity: cache_capacity,
                    entries: HashMap::new(),
                    order: VecDeque::new(),
                }),
                concurrency: Arc::new(Semaphore::new(max_concurrency)),
                metrics: ServiceMetrics::new(
                    "tax_engine",
                    &["calculate", "calculate_document", "runtime_reload"],
                )
                .map_err(|error| error.to_string())?,
                cache_capacity,
            }),
        })
    }

    pub fn cache_capacity(&self) -> usize {
        self.inner.cache_capacity
    }

    pub fn metrics(&self) -> ServiceMetrics {
        self.inner.metrics.clone()
    }

    #[cfg(test)]
    fn cache_len(&self) -> usize {
        self.inner
            .cache
            .lock()
            .map(|cache| cache.entries.len())
            .unwrap_or_default()
    }

    pub fn router(&self, auth_token: &str) -> Router {
        self.router_with_auth_token(Arc::new(ArcSwap::from_pointee(auth_token.to_string())))
    }

    pub fn router_with_auth_token(&self, auth_token: Arc<ArcSwap<String>>) -> Router {
        let state = AppState {
            engine: self.clone(),
            auth_token,
        };
        Router::new()
            .route("/healthz", get(healthz))
            .route("/metrics", get(metrics))
            .merge(
                Router::new()
                    .route("/readyz", get(readyz))
                    .route("/v1/calculate", post(calculate))
                    .route("/v1/calculate-document", post(calculate_document))
                    .layer(middleware::from_fn_with_state(state.clone(), authenticate)),
            )
            .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
            .with_state(state)
    }

    async fn calculate(
        &self,
        request: CalculateRequest,
    ) -> Result<CalculationResult, ServiceError> {
        let permit = self.acquire().await?;
        let service = self.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let started = Instant::now();
            let result = service.calculate_blocking(request);
            service
                .inner
                .metrics
                .observe_or_log("calculate", started.elapsed(), result.is_err());
            result
        })
        .await
        .map_err(|_| ServiceError::WorkerFailed)?
    }

    async fn calculate_document(
        &self,
        request: CalculateDocumentRequest,
    ) -> Result<TaxDocumentResult, ServiceError> {
        let permit = self.acquire().await?;
        let service = self.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let started = Instant::now();
            let result = service.calculate_document_blocking(request);
            service.inner.metrics.observe_or_log(
                "calculate_document",
                started.elapsed(),
                result.is_err(),
            );
            result
        })
        .await
        .map_err(|_| ServiceError::WorkerFailed)?
    }

    async fn acquire(&self) -> Result<OwnedSemaphorePermit, ServiceError> {
        tokio::time::timeout(
            MAX_QUEUE_WAIT,
            self.inner.concurrency.clone().acquire_owned(),
        )
        .await
        .map_err(|_| ServiceError::Overloaded)?
        .map_err(|_| ServiceError::Overloaded)
    }

    fn calculate_blocking(
        &self,
        request: CalculateRequest,
    ) -> Result<CalculationResult, ServiceError> {
        let compiled = self.compiled(&request.rule_set)?;
        compiled
            .calculate(&request.facts)
            .map_err(|error| ServiceError::InvalidRequest(error.to_string()))
    }

    fn calculate_document_blocking(
        &self,
        request: CalculateDocumentRequest,
    ) -> Result<TaxDocumentResult, ServiceError> {
        let compiled = self.compiled(&request.rule_set)?;
        compiled
            .calculate_document(&request.facts)
            .map_err(|error| ServiceError::InvalidRequest(error.to_string()))
    }

    fn compiled(&self, document: &RuleSetDocument) -> Result<Arc<CompiledRuleSet>, ServiceError> {
        document
            .validate()
            .map_err(|error| ServiceError::InvalidRequest(error.to_string()))?;
        let digest = document
            .rule_digest()
            .map_err(|error| ServiceError::InvalidRequest(error.to_string()))?;
        if let Ok(mut cache) = self.inner.cache.lock()
            && let Some(compiled) = cache.entries.get(&digest).cloned()
        {
            touch(&mut cache.order, digest);
            return Ok(compiled);
        }
        let compiled = Arc::new(
            document
                .rule_set
                .compile()
                .map_err(|error| ServiceError::InvalidRequest(error.to_string()))?,
        );
        let mut cache = self
            .inner
            .cache
            .lock()
            .map_err(|_| ServiceError::WorkerFailed)?;
        if cache.entries.len() >= cache.capacity
            && let Some(oldest) = cache.order.pop_front()
        {
            cache.entries.remove(&oldest);
        }
        cache.entries.insert(digest, compiled.clone());
        cache.order.push_back(digest);
        Ok(compiled)
    }
}

fn touch(order: &mut VecDeque<[u8; 32]>, digest: [u8; 32]) {
    if let Some(index) = order.iter().position(|value| *value == digest) {
        order.remove(index);
    }
    order.push_back(digest);
}

#[derive(Clone)]
struct AppState {
    engine: EngineService,
    auth_token: Arc<ArcSwap<String>>,
}

async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn readyz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn metrics(State(state): State<AppState>) -> Response {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        state.engine.inner.metrics.render_prometheus(),
    )
        .into_response()
}

async fn calculate(
    State(state): State<AppState>,
    payload: Result<Json<CalculateRequest>, JsonRejection>,
) -> Result<Json<CalculationResult>, ServiceError> {
    let Json(request) = payload.map_err(map_json_rejection)?;
    Ok(Json(state.engine.calculate(request).await?))
}

async fn calculate_document(
    State(state): State<AppState>,
    payload: Result<Json<CalculateDocumentRequest>, JsonRejection>,
) -> Result<Json<TaxDocumentResult>, ServiceError> {
    let Json(request) = payload.map_err(map_json_rejection)?;
    Ok(Json(state.engine.calculate_document(request).await?))
}

fn map_json_rejection(rejection: JsonRejection) -> ServiceError {
    if let JsonRejection::BytesRejection(BytesRejection::FailedToBufferBody(
        FailedToBufferBody::LengthLimitError(_),
    )) = rejection
    {
        return ServiceError::PayloadTooLarge;
    }
    ServiceError::InvalidRequest("request body is invalid".to_string())
}

async fn authenticate(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let auth_token = state.auth_token.load();
    match authorize(request.headers(), auth_token.as_str()) {
        Ok(()) => next.run(request).await,
        Err(error) => error.into_response(),
    }
}

fn authorize(headers: &HeaderMap, expected: &str) -> Result<(), ServiceError> {
    let value = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ServiceError::Unauthorized)?;
    let token = value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty() && token.len() <= 256)
        .ok_or(ServiceError::Unauthorized)?;
    if !bool::from(token.as_bytes().ct_eq(expected.as_bytes())) {
        return Err(ServiceError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CalculateRequest, EngineService, MAX_REQUEST_BODY_BYTES};
    use arc_swap::ArcSwap;
    use axum::{body::Body, http::Request, http::StatusCode};
    use std::sync::Arc;
    use tax_engine::{
        BaseReference, PriceMode, Rate, RateSpec, RoundingMode, RoundingScope, RoundingSpec,
        RuleSet, RuleSetDocument, TaxBase, TaxComponentRule, TaxFacts,
    };
    use tower::ServiceExt;

    fn request() -> CalculateRequest {
        CalculateRequest {
            rule_set: RuleSetDocument::new(RuleSet {
                id: "service-test".to_string(),
                version: 1,
                components: vec![TaxComponentRule {
                    id: "sales-tax".to_string(),
                    label: "Sales tax".to_string(),
                    base: TaxBase::single(BaseReference::Net),
                    rate: RateSpec::Flat(Rate::new(10, 100).expect("valid rate")),
                    price_mode: PriceMode::Exclusive,
                    credit_atomic: 0,
                    floor_atomic: None,
                    cap_atomic: None,
                    rounding: RoundingSpec {
                        mode: RoundingMode::HalfUp,
                        scope: RoundingScope::Component,
                        quantum_atomic: 1,
                    },
                    threshold: None,
                    exemption_codes: Vec::new(),
                }],
            }),
            facts: TaxFacts {
                gross_atomic: 1_000,
                net_atomic: 1_000,
                currency: "USD".to_string(),
                jurisdiction: "US-CA".to_string(),
                jurisdiction_resolution: None,
                fx_conversion: None,
                exemption_codes: Vec::new(),
                cumulative_period: None,
            },
        }
    }

    #[test]
    fn bounded_cache_contract_is_executable() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        assert_eq!(service.cache_capacity(), 2);
    }

    #[tokio::test]
    async fn protected_calculation_route_requires_service_authentication() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        let response = service
            .router("local-development-only")
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/v1/calculate")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_token_snapshot_rotates_without_rebuilding_the_router() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        let token = Arc::new(ArcSwap::from_pointee("old-token".to_string()));
        let router = service.router_with_auth_token(token.clone());

        let old_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/readyz")
                    .header("authorization", "Bearer old-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("old token response");
        assert_eq!(old_response.status(), StatusCode::NO_CONTENT);

        token.store(Arc::new("new-token".to_string()));

        let old_after_rotation = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/readyz")
                    .header("authorization", "Bearer old-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("old token response after rotation");
        assert_eq!(old_after_rotation.status(), StatusCode::UNAUTHORIZED);

        let new_response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/readyz")
                    .header("authorization", "Bearer new-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("new token response");
        assert_eq!(new_response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn internal_metrics_plane_is_scrapeable_without_calculation_credentials() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        let response = service
            .router("local-development-only")
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            axum::body::to_bytes(response.into_body(), MAX_REQUEST_BODY_BYTES)
                .await
                .expect("body")
                .windows(b"stp_requests_total".len())
                .any(|window| window == b"stp_requests_total")
        );
    }

    #[tokio::test]
    async fn authenticated_calculation_is_deterministic_and_warms_bounded_cache() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        let payload = serde_json::to_vec(&request()).expect("request serializes");
        let response = service
            .router("local-development-only")
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/calculate")
                    .header("authorization", "Bearer local-development-only")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.clone()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let first: tax_engine::CalculationResult = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), MAX_REQUEST_BODY_BYTES)
                .await
                .expect("body"),
        )
        .expect("calculation result");
        assert_eq!(first.total_tax_atomic, 100);
        assert_eq!(service.cache_len(), 1);

        let response = service
            .router("local-development-only")
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/calculate")
                    .header("authorization", "Bearer local-development-only")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let second: tax_engine::CalculationResult = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), MAX_REQUEST_BODY_BYTES)
                .await
                .expect("body"),
        )
        .expect("calculation result");
        assert_eq!(second, first);
        assert_eq!(service.cache_len(), 1);
    }

    #[tokio::test]
    async fn malformed_json_is_a_bad_request_after_authentication() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        let response = service
            .router("local-development-only")
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/calculate")
                    .header("authorization", "Bearer local-development-only")
                    .header("content-type", "application/json")
                    .body(Body::from("{not-json"))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn cache_evicts_oldest_rule_digest_at_the_declared_bound() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        let mut first = request();
        first.rule_set.rule_set.id = "first".to_string();
        let mut second = request();
        second.rule_set.rule_set.id = "second".to_string();
        let mut third = request();
        third.rule_set.rule_set.id = "third".to_string();
        service
            .calculate_blocking(first)
            .expect("first calculation");
        service
            .calculate_blocking(second)
            .expect("second calculation");
        service
            .calculate_blocking(third)
            .expect("third calculation");
        assert_eq!(service.cache_len(), 2);
    }

    #[tokio::test]
    async fn request_body_limit_is_enforced_after_authentication() {
        let service = EngineService::new(2).expect("bounded cache configuration");
        let response = service
            .router("local-development-only")
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/calculate")
                    .header("authorization", "Bearer local-development-only")
                    .header("content-type", "application/json")
                    .body(Body::from(vec![b' '; MAX_REQUEST_BODY_BYTES + 1]))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn limits_reject_unbounded_configuration() {
        assert!(EngineService::new(0).is_err());
        assert!(EngineService::with_limits(1, 0).is_err());
        assert!(EngineService::with_limits(257, 1).is_err());
        assert!(EngineService::with_limits(1, 257).is_err());
    }

    #[allow(dead_code)]
    fn request_type_is_exported() {
        let _ = std::mem::size_of::<CalculateRequest>();
    }
}
