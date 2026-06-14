-- V025: Cartridge → Printer device link (D-PRN07-01).
--
-- Adds `current_printer_device_id` to cartridges so PrinterDetail can show
-- which cartridge is currently installed in a printer (FK to devices, not printers,
-- because the canonical device identity is in the `devices` table per D-Schema-01).
--
-- PrinterService.install_cartridge sets:   cartridges.current_printer_device_id = printer.device_id
-- PrinterService.remove_cartridge clears:  cartridges.current_printer_device_id = NULL
-- PrinterRepository.current_cartridge_for_printer(printer_device_id):
--   SELECT id FROM cartridges WHERE current_printer_device_id = ?1 LIMIT 1

ALTER TABLE cartridges ADD COLUMN current_printer_device_id INTEGER NULL REFERENCES devices(id);

PRAGMA user_version = 25;
