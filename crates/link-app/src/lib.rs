//! Trackly Link: библиотека клиента. Все модули объявлены здесь заранее (интерфейс-первым):
//! планы-владельцы заполняют свои файлы и lib.rs не правят.

pub mod authflow;
pub mod autostart;
pub mod backoff;
pub mod commands;
pub mod config;
pub mod connection;
pub mod enroll;
pub mod http_client;
pub mod pin;
pub mod runtime;
pub mod secrets;
pub mod sso;
pub mod state;
pub mod tray;
pub mod window_policy;
pub mod ws_client;
