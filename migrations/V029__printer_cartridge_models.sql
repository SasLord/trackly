-- V029: Printer ↔ cartridge-model compatibility junction table (D-11/D-12,
-- Phase 12 gap closure — GAP-12-02 backend half).
--
-- Links a printer (device with type_id = 2) to compatible cartridge models
-- (cartridge_models with kind_id = 1) by ID — distinct from the existing
-- free-text `cartridge_model_compatibility` table (V005, printer_brand +
-- printer_model TEXT columns), which is NOT touched by this migration and
-- remains the source of truth for the brand/model autocomplete editor.
--
-- Editable from BOTH sides (D-12): the printer card writes via
-- SqlitePrinterRepository::set_compatible_models_in_tx(device_id, model_ids),
-- the cartridge-model card writes via
-- SqlitePrinterRepository::set_compatible_devices_in_tx(cartridge_model_id, device_ids).
-- Both mutate the SAME table — DELETE+re-INSERT per call, no soft-delete,
-- no version column (pure link table, mirrors upsert_compatibility_in_tx's
-- pattern for the existing free-text table).
--
-- Empty link set for a given device_id means "compatibility not configured"
-- (D-14) — CartridgeRepository::list()'s compatible_with_printer_device_id
-- filter must pass through unfiltered in that case, not exclude everything.

CREATE TABLE printer_cartridge_models (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id          INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    cartridge_model_id INTEGER NOT NULL REFERENCES cartridge_models(id) ON DELETE CASCADE,
    created_at_utc     INTEGER NOT NULL
);

-- Prevents duplicate links; also covers the device_id-prefix lookup used by
-- the printer-side filter subquery in CartridgeRepository::list().
CREATE UNIQUE INDEX idx_printer_cartridge_models_unique
    ON printer_cartridge_models(device_id, cartridge_model_id);

-- Reverse-lookup index for the cartridge-model-side editor
-- (WHERE cartridge_model_id = ?).
CREATE INDEX idx_printer_cartridge_models_model
    ON printer_cartridge_models(cartridge_model_id);

PRAGMA user_version = 29;
