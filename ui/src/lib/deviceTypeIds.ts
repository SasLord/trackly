// Id типов устройств (сид V001) — единственный фронтовый источник признака
// «это принтер» (INT-31, quick-261121-heh). Зеркало trackly-core
// `crates/trackly-core/src/domain/devices.rs` (`DEVICE_TYPE_ID`,
// `PRINTER_TYPE_ID`, `is_printer_type`); значение сверяется с Rust гейтом
// `scripts/check-printer-identity.mjs` [J4]. Подписи типов — НЕ здесь: они
// данные справочника `device_types` (стор deviceTypes, 41.12 D-01).

/** `device_types.id` типа «Устройство». */
export const DEVICE_TYPE_ID = 1;

/** `device_types.id` типа «Принтер» — единственный признак принтера. */
export const PRINTER_TYPE_ID = 2;

/** Признак «это принтер» по `type_id` устройства. */
export function isPrinterType(typeId: number | null | undefined): boolean {
  return typeId === PRINTER_TYPE_ID;
}
