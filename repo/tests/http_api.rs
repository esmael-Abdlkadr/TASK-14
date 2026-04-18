//! HTTP integration tests against a **live** backend (`CIVICSORT_API_URL`, default http://127.0.0.1:8080).
//! No mock HTTP servers: all requests are real TCP to the running API (see `common` / `api_surface`).
//! Run: `docker-compose up -d` then `cargo test --tests` or `./run_tests.sh api`.

#[macro_use]
mod common;
mod api_surface;
mod health;
mod route_catalog;
