//! Общие сидеры тестов ремонтных заявок (Phase 41.4). Имена вымышленные.
#![allow(dead_code)]

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

pub async fn seed_user(ctx: &AppCtx, login: &str, full: &str) -> i64 {
    let (login, full) = (login.to_string(), full.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, 'employee', 1700000000, 1700000000)",
                params![login, full],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed user")
}

pub fn emp(id: i64) -> Identity {
    Identity {
        user_id: Some(id),
        role: Role::Employee,
    }
}

pub fn manager(id: i64) -> Identity {
    Identity {
        user_id: Some(id),
        role: Role::Manager,
    }
}

/// Группа с пользователем-владельцем (свойство типа `users`) и устройствами в составе:
/// область сотрудника в `employee_device_ids`.
pub async fn seed_user_group(ctx: &AppCtx, user_id: i64, label: &str, devices: &[i64]) -> i64 {
    let label = label.to_string();
    let devices = devices.to_vec();
    ctx.writer
        .execute(move |conn| {
            let code = format!("arm_t_{user_id}");
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, 'workstation', 1700000000, 1700000000)",
                params![code, format!("АРМ-тест {label}")],
            )
            .map_err(map_rusqlite)?;
            let t = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO group_type_properties (type_id, name, data_type, created_at_utc, \
                 updated_at_utc) VALUES (?1, 'Пользователи', 'users', 1700000000, 1700000000)",
                params![t],
            )
            .map_err(map_rusqlite)?;
            let prop = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, version, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, 1, 1, 1700000000, 1700000000)",
                params![t, label],
            )
            .map_err(map_rusqlite)?;
            let g = conn.last_insert_rowid();
            for d in devices {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![d, g],
                )
                .map_err(map_rusqlite)?;
            }
            conn.execute(
                "INSERT INTO group_property_values (group_id, property_id, value_ref, \
                 updated_at_utc) VALUES (?1, ?2, ?3, 1700000000)",
                params![g, prop, user_id],
            )
            .map_err(map_rusqlite)?;
            Ok(g)
        })
        .await
        .expect("seed user group")
}

pub async fn count(ctx: &AppCtx, sql: &'static str, arg: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![arg], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("count")
}
