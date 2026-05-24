//! Plan 05 stub — здесь будет `impl axum::response::IntoResponse for AppError`.
//!
//! Plan 04 владеет полным `AppError` enum в `trackly-core`; Plan 05 добавит
//! HTTP-маппинг (NotFound→404, Conflict→409, OptimisticLockMismatch→409,
//! WriteQueueBusy→503, DatabaseFromNewerVersion→500, Validation→400,
//! Unauthorized→401, Forbidden→403, Internal→500) и подключит axum-роутер.
