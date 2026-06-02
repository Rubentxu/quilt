//! Bearer token authentication middleware
//!
//! Protects API routes with a simple shared-secret Bearer token.
//! The token is configured once at startup via [`init`] and checked
//! against the `Authorization: Bearer <token>` header on each request.
//!
//! # RFC 7235 §2.1 compliance
//!
//! The auth-scheme is parsed **case-insensitively**, so `Bearer`,
//! `bearer`, `BEARER`, and `BeArEr` are all accepted. Extra
//! whitespace around the token is also tolerated.
//!
//! # Public routes (no auth required)
//!
//! - `/health` and `/api/v1/health` — health checks
//! - `/metrics` — Prometheus metrics
//! - `/ws` — WebSocket upgrade (read-only navigation events)
//! - `/` and `/*path` — frontend static assets (SPA)
//!
//! # Protected routes (auth required)
//!
//! - `/api/v1/*` — all REST API endpoints

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::OnceLock;
use tracing::warn;

/// The shared API key checked by the auth middleware.
static API_KEY: OnceLock<String> = OnceLock::new();

/// Initialize the shared API key.
///
/// Must be called **once** at server startup **before** the listener starts.
/// Panics if called more than once.
pub fn init(api_key: String) {
    API_KEY
        .set(api_key)
        .expect("auth::init called more than once");
}

/// Supported authentication schemes.
///
/// Open for extension: add a variant here (and a match arm in
/// [`parse_auth_header`]) to support a new scheme such as `Basic` or
/// `Digest`. The middleware itself stays untouched — it only sees the
/// [`AuthScheme::token`] view.
#[derive(Debug, PartialEq, Eq)]
enum AuthScheme {
    /// `Authorization: Bearer <token>` (RFC 6750).
    Bearer(String),
    // Future: Basic(String), Digest(...), etc.
}

impl AuthScheme {
    /// The opaque credentials portion of the Authorization header.
    fn token(&self) -> &str {
        match self {
            AuthScheme::Bearer(token) => token,
        }
    }
}

/// Parses the `Authorization` header value into an [`AuthScheme`].
///
/// * **Case-insensitive scheme** — RFC 7235 §2.1 says `auth-scheme` is
///   case-insensitive, so `Bearer`, `bearer`, `BEARER` are all valid.
/// * **Whitespace-tolerant** — leading / trailing whitespace on the
///   header value and extra spaces between scheme and token are trimmed.
/// * **Unknown schemes return `None`** so the caller can fall through
///   to a 401.
fn parse_auth_header(value: &str) -> Option<AuthScheme> {
    let trimmed = value.trim();
    let (scheme, rest) = trimmed.split_once(' ')?;

    // RFC 7235 §2.1: auth-scheme is case-insensitive
    match scheme.to_ascii_lowercase().as_str() {
        "bearer" => Some(AuthScheme::Bearer(rest.trim().to_string())),
        // Future: "basic" => Some(AuthScheme::Basic(rest.trim().to_string())),
        _ => None,
    }
}

/// Axum middleware that enforces Bearer token authentication.
///
/// # Flow
///
/// 1. Public-path check — health, metrics, frontend, and WebSocket skip auth.
/// 2. OPTIONS preflight — always allowed through for CORS.
/// 3. Bearer token check — `Authorization: Bearer <token>` must match the
///    configured API key. Returns `401 Unauthorized` on mismatch.
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let method = request.method();

    // ---- Public routes ----
    // Health checks
    if path == "/health" || path == "/api/v1/health" {
        return next.run(request).await;
    }

    // Metrics endpoint (operational, not user-facing)
    if path == "/metrics" {
        return next.run(request).await;
    }

    // WebSocket endpoint (read-only navigation events)
    if path == "/ws" {
        return next.run(request).await;
    }

    // Frontend SPA root
    if path == "/" {
        return next.run(request).await;
    }

    // Non-API, non-WS paths are treated as frontend static assets → public
    if !path.starts_with("/api/") && !path.starts_with("/ws") {
        return next.run(request).await;
    }

    // ---- CORS preflight ----
    // Let OPTIONS through so the CorsLayer can handle it without auth
    if method == axum::http::Method::OPTIONS {
        return next.run(request).await;
    }

    // ---- Auth check ----
    let expected = API_KEY.get().expect("auth::init not called");

    let valid = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_auth_header)
        .is_some_and(|scheme| scheme.token() == expected);

    if valid {
        next.run(request).await
    } else {
        warn!("Unauthorized request: {method} {path}");
        (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body, http::Request, middleware::from_fn, response::IntoResponse, routing::get,
        Router,
    };
    use std::sync::Once;
    use tower::util::ServiceExt;

    static INIT: Once = Once::new();

    /// Initialize the static API_KEY once for all tests.
    fn init_test_key() {
        INIT.call_once(|| init("test-key-123".to_string()));
    }

    fn test_app() -> Router {
        init_test_key();
        Router::new()
            .route("/api/v1/pages", get(test_handler))
            .route("/health", get(test_handler))
            .route("/metrics", get(test_handler))
            .route("/ws", get(test_handler))
            .route("/", get(test_handler))
            .route("/some-asset.js", get(test_handler))
            .layer(from_fn(auth_middleware))
    }

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "OK")
    }

    // ---------------------------------------------------------------------------
    // parse_auth_header — pure-function unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn parse_lowercase_bearer_scheme() {
        assert_eq!(
            parse_auth_header("bearer abc"),
            Some(AuthScheme::Bearer("abc".to_string()))
        );
    }

    #[test]
    fn parse_uppercase_bearer_scheme() {
        assert_eq!(
            parse_auth_header("BEARER abc"),
            Some(AuthScheme::Bearer("abc".to_string()))
        );
    }

    #[test]
    fn parse_mixed_case_bearer_scheme() {
        assert_eq!(
            parse_auth_header("BeArEr abc"),
            Some(AuthScheme::Bearer("abc".to_string()))
        );
    }

    #[test]
    fn parse_extra_whitespace_around_header() {
        assert_eq!(
            parse_auth_header("  Bearer   abc  "),
            Some(AuthScheme::Bearer("abc".to_string()))
        );
    }

    #[test]
    fn parse_unknown_scheme_returns_none() {
        assert_eq!(parse_auth_header("Basic xyz"), None);
    }

    #[test]
    fn parse_missing_scheme_returns_none() {
        // No space at all → not a valid scheme/credentials pair.
        assert_eq!(parse_auth_header("abc"), None);
    }

    #[test]
    fn parse_bearer_with_only_trailing_whitespace_returns_none() {
        // "Bearer " is fully trimmed to "Bearer" by the leading `.trim()`,
        // so there's no space left to split on → None.
        // The middleware still 401s on this input via the
        // `empty_bearer_token_returns_401` integration test below.
        assert_eq!(parse_auth_header("Bearer "), None);
    }

    #[test]
    fn parse_empty_value_returns_none() {
        assert_eq!(parse_auth_header(""), None);
    }

    #[test]
    fn parse_whitespace_only_returns_none() {
        assert_eq!(parse_auth_header("   "), None);
    }

    // ---------------------------------------------------------------------------
    // Middleware integration tests — public routes
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn health_route_bypasses_auth() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_v1_health_bypasses_auth() {
        // The middleware passes it through, but there's no handler for /api/v1/health
        // in this test router — so we get 404 instead of auth's 401.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn metrics_bypasses_auth() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ws_bypasses_auth() {
        let res = test_app()
            .oneshot(Request::builder().uri("/ws").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn frontend_root_bypasses_auth() {
        let res = test_app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn frontend_asset_bypasses_auth() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/some-asset.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // ---------------------------------------------------------------------------
    // Middleware integration tests — auth check
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn api_route_with_valid_token_succeeds() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Bearer test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_route_without_token_returns_401() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_route_with_wrong_token_returns_401() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Bearer wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_route_with_basic_scheme_returns_401() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Basic test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn options_preflight_bypasses_auth() {
        // The middleware passes OPTIONS through without auth.
        // This test router has no OPTIONS handler so Axum returns 405.
        // In production the CorsLayer handles preflight before auth.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::OPTIONS)
                    .uri("/api/v1/pages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // ---------------------------------------------------------------------------
    // Edge case tests
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn empty_bearer_token_returns_401() {
        // "Bearer " → empty token, never matches the configured key.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Bearer ")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn token_in_query_param_returns_401() {
        // Query parameters are NOT a valid auth mechanism — only the header is checked.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages?token=test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // ----- RFC 7235 §2.1 — case-insensitive scheme -----

    #[tokio::test]
    async fn lowercase_bearer_scheme_accepted() {
        // RFC 7235 §2.1: auth-scheme is case-insensitive.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "bearer test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn uppercase_bearer_scheme_accepted() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "BEARER test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn mixed_case_bearer_scheme_accepted() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "BeArEr test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn extra_whitespace_around_token_accepted() {
        // Leading and trailing whitespace on the header value is trimmed.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "  Bearer test-key-123  ")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn double_space_between_scheme_and_token_accepted() {
        // Previously rejected by strip_prefix("Bearer ") (matches exactly one space).
        // parse_auth_header trims internal whitespace — now accepted.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Bearer  test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn trailing_space_in_token_accepted() {
        // rest.trim() in parse_auth_header drops the trailing space, recovering the key.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Bearer test-key-123 ")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn multiple_authorization_headers_uses_first() {
        // HTTP allows multiple headers with the same name; .get() returns the first.
        // If the first is wrong the request fails even if the second is correct.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Bearer wrong-key")
                    .header(header::AUTHORIZATION, "Bearer test-key-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn very_long_token_rejected() {
        let long_token = "x".repeat(1000);
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, format!("Bearer {long_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn token_with_special_chars_rejected() {
        // Dots and equals in token value — should be handled without error.
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages")
                    .header(header::AUTHORIZATION, "Bearer token.with.dots=and.equals")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn health_subpaths_bypass_auth() {
        // /health/live and /health/ready don't match the exact /health check,
        // but they also don't start with /api/ so they fall through to the
        // "frontend static asset" rule and bypass auth.
        // The test router has no handler for these paths → Axum returns 404,
        // but importantly it does NOT return 401 (auth was bypassed).
        for path in ["/health/live", "/health/ready"] {
            let res = test_app()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_ne!(
                res.status(),
                StatusCode::UNAUTHORIZED,
                "path {path} should bypass auth (got {})",
                res.status()
            );
        }
    }

    #[tokio::test]
    async fn post_api_route_requires_auth() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/api/v1/pages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Middleware returns 401 before the router sees POST (no handler registered).
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn delete_api_route_requires_auth() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::DELETE)
                    .uri("/api/v1/pages/some-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_subpath_requires_auth() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pages/foo/blocks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_blocks_subpath_requires_auth() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/blocks/abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn concurrent_auth_checks() {
        // 100 concurrent requests with a mix of valid and missing tokens.
        // The shared OnceLock is read-only after init so there's no race.
        let app = test_app();
        let mut handles = Vec::with_capacity(100);

        for i in 0..100 {
            let app = app.clone();
            handles.push(tokio::spawn(async move {
                if i % 2 == 0 {
                    app.oneshot(
                        Request::builder()
                            .uri("/api/v1/pages")
                            .header(header::AUTHORIZATION, "Bearer test-key-123")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap()
                    .status()
                } else {
                    app.oneshot(
                        Request::builder()
                            .uri("/api/v1/pages")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap()
                    .status()
                }
            }));
        }

        for (i, handle) in handles.into_iter().enumerate() {
            let status = handle.await.unwrap();
            if i % 2 == 0 {
                assert_eq!(status, StatusCode::OK, "even {i} should be 200");
            } else {
                assert_eq!(status, StatusCode::UNAUTHORIZED, "odd {i} should be 401");
            }
        }
    }
}
