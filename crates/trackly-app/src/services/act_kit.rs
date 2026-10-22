//! Комплектация якорного устройства группы для печатной формы акта (Phase 41.2, D-11, D-12).
//!
//! В акт уходит одна позиция — якорь группы; детали отдельных позиций не получают, поэтому
//! их имена и номера собираются в строку комплектации якоря. Строка хранится в
//! `act_items.complectation_at_time` как текст: экранирование делает шаблонизатор
//! при печати, шаблоны и переменные `templates/*.html` не меняются.

use rusqlite::Transaction;
use trackly_core::domain::devices::DeviceRow;
use trackly_core::error::AppError;
use trackly_infra::repos::{SqliteDeviceRepository, SqliteGroupRepository};

/// Деталь якорной группы в том виде, в каком она попадает в комплектацию.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KitPart {
    pub name: String,
    pub inventory_number: Option<String>,
    pub serial_number: Option<String>,
}

fn clean(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}

fn format_part(part: &KitPart) -> String {
    let name = part.name.trim();
    if let Some(inv) = clean(part.inventory_number.as_deref()) {
        format!("{name} (инв. {inv})")
    } else if let Some(serial) = clean(part.serial_number.as_deref()) {
        format!("{name} (с/н {serial})")
    } else {
        name.to_string()
    }
}

/// Строка комплектации якоря: собственная комплектация первой, затем детали через `; `.
///
/// У детали печатается один номер: инвентарный, иначе серийный, иначе без скобок.
/// Порядок деталей функция не меняет: его задаёт вызывающий.
pub(crate) fn format_anchor_kit(own_kit: Option<&str>, parts: &[KitPart]) -> Option<String> {
    let own = clean(own_kit);
    if parts.is_empty() {
        return own.and(own_kit).map(str::to_string);
    }
    let mut chunks: Vec<String> = Vec::with_capacity(parts.len() + 1);
    if let Some(own) = own {
        chunks.push(own.to_string());
    }
    chunks.extend(parts.iter().map(format_part));
    Some(chunks.join("; "))
}

/// Комплектация для позиции акта по устройству: для якоря группы собирается из деталей,
/// для остальных возвращается собственная комплектация без изменений.
///
/// Детали упорядочиваются по `(имя, id)` побайтово в Rust, а не коллацией SQL, поэтому
/// порядок не зависит от платформы. Одна функция обслуживает все пути создания позиции акта.
pub(crate) fn kit_for_item_in_tx(
    tx: &Transaction<'_>,
    devices: &SqliteDeviceRepository,
    groups: &SqliteGroupRepository,
    device: &DeviceRow,
) -> Result<Option<String>, AppError> {
    let Some(group) = groups.anchor_group_of_device_in_tx(tx, device.id)? else {
        return Ok(device.kit.clone());
    };
    let mut rows: Vec<DeviceRow> = Vec::new();
    for id in groups.subtree_device_ids_in_tx(tx, group.id)? {
        if id == device.id {
            continue;
        }
        rows.push(devices.get_in_tx(tx, id)?);
    }
    rows.sort_by(|a, b| {
        a.name
            .trim()
            .as_bytes()
            .cmp(b.name.trim().as_bytes())
            .then(a.id.cmp(&b.id))
    });
    let parts: Vec<KitPart> = rows
        .into_iter()
        .map(|r| KitPart {
            name: r.name,
            inventory_number: r.inventory_no,
            serial_number: r.serial_no,
        })
        .collect();
    Ok(format_anchor_kit(device.kit.as_deref(), &parts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, inv: Option<&str>, serial: Option<&str>) -> KitPart {
        KitPart {
            name: name.to_string(),
            inventory_number: inv.map(str::to_string),
            serial_number: serial.map(str::to_string),
        }
    }

    #[test]
    fn golden_kit_line_matches_exactly() {
        let parts = [
            part("SSD Kingston 480ГБ", Some("114"), None),
            part("БП 500W", None, None),
            part("ОЗУ 8ГБ", None, Some("AB123")),
        ];
        assert_eq!(
            format_anchor_kit(Some("Корпус Mid Tower"), &parts).as_deref(),
            Some("Корпус Mid Tower; SSD Kingston 480ГБ (инв. 114); БП 500W; ОЗУ 8ГБ (с/н AB123)")
        );
    }

    #[test]
    fn inventory_number_wins_over_serial() {
        let parts = [part("Монитор #2", Some("77"), Some("SN-1"))];
        assert_eq!(
            format_anchor_kit(None, &parts).as_deref(),
            Some("Монитор #2 (инв. 77)")
        );
    }

    #[test]
    fn blank_numbers_count_as_absent_and_edges_are_trimmed() {
        let parts = [
            part("  Клавиатура #5  ", Some("   "), Some("  S9 ")),
            part("Мышь", Some(""), Some(" ")),
        ];
        assert_eq!(
            format_anchor_kit(Some("  "), &parts).as_deref(),
            Some("Клавиатура #5 (с/н S9); Мышь")
        );
    }

    #[test]
    fn without_own_kit_the_line_starts_with_first_part() {
        let parts = [part("БП 500W", None, None)];
        assert_eq!(format_anchor_kit(None, &parts).as_deref(), Some("БП 500W"));
        assert_eq!(
            format_anchor_kit(Some(""), &parts).as_deref(),
            Some("БП 500W")
        );
    }

    #[test]
    fn without_parts_own_kit_is_returned_as_is() {
        assert_eq!(
            format_anchor_kit(Some("Корпус"), &[]).as_deref(),
            Some("Корпус")
        );
        assert_eq!(format_anchor_kit(None, &[]), None);
        assert_eq!(format_anchor_kit(Some("  "), &[]), None);
    }

    #[test]
    fn markup_characters_are_stored_unescaped() {
        let parts = [part("Кулер R&D <v2>", None, None)];
        assert_eq!(
            format_anchor_kit(None, &parts).as_deref(),
            Some("Кулер R&D <v2>")
        );
    }
}
