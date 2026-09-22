//! BE-CR-04 (D-14, "перечислять от сервера") — registry gate: every server
//! mutation that changes a numbering space or the templates/context memory
//! behind the «Вставка» menus must broadcast `WsEvent::NumberSpaceChanged`
//! for the affected contexts. Each case below runs one mutation against
//! services wired to a shared broadcast channel and asserts the event.
//!
//! When adding a new method that writes `devices.inventory_number`,
//! `acts.number` (or changes a return's displayed number), `cartridges.code`,
//! `number_templates*`, add a case here. Fictional data only.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::dto::device::DeviceNew;
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto, TemplateTypeDto};
use trackly_app::dto::printer::WsEvent;
use trackly_app::services::{ActService, CartridgeService, DeviceService, NumberTemplateService};
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

struct Fixture {
    nts: NumberTemplateService,
    devices: DeviceService,
    cartridges: CartridgeService,
    acts: ActService,
    rx: broadcast::Receiver<WsEvent>,
    _dir: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (tx, rx) = broadcast::channel::<WsEvent>(256);
    let tx = Arc::new(tx);
    Fixture {
        nts: NumberTemplateService::new(writer.clone(), readers.clone(), clock.clone())
            .with_ws_tx(tx.clone()),
        devices: DeviceService::new(writer.clone(), readers.clone(), clock.clone())
            .with_ws_tx(tx.clone()),
        cartridges: CartridgeService::new(writer.clone(), readers.clone(), clock.clone())
            .with_ws_tx(tx.clone()),
        acts: ActService::new(writer, readers, clock).with_ws_tx(tx),
        rx,
        _dir: dir,
    }
}

fn number_input(value: &str) -> NumberFieldInput {
    NumberFieldInput {
        value: value.to_string(),
        template_id: None,
        confirm_mismatch: false,
        confirm_script_mix: false,
    }
}

fn device_new(inventory_no: Option<&str>) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: "Ноутбук".to_string(),
        inventory_no: inventory_no.map(str::to_string),
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: None,
        status_id: 1,
    }
}

/// Drain everything received so far.
fn drain(rx: &mut broadcast::Receiver<WsEvent>) {
    while rx.try_recv().is_ok() {}
}

/// Assert that a `NumberSpaceChanged` containing `context` was sent since
/// the last drain.
fn assert_broadcast(rx: &mut broadcast::Receiver<WsEvent>, context: &str, what: &str) {
    let mut seen = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let WsEvent::NumberSpaceChanged { contexts } = ev {
            if contexts.iter().any(|c| c == context) {
                return;
            }
            seen.push(contexts);
        }
    }
    panic!("{what}: expected NumberSpaceChanged with {context:?}, saw {seen:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_number_space_mutation_broadcasts() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let mut f = fixture();

        // --- template CRUD + context memory ---------------------------------
        drain(&mut f.rx);
        let t = f
            .nts
            .create(TemplateTypeDto::DeviceInventory, "ИНВ-[XXXX]".into())
            .await
            .expect("template create");
        assert_broadcast(&mut f.rx, "device_create", "template create");

        let t = f
            .nts
            .update_mask(t.id, "ОРГ-[XXXX]".into(), t.version)
            .await
            .expect("template update");
        assert_broadcast(&mut f.rx, "printer_create", "template update_mask");

        f.nts
            .remember_context(TemplateContextDto::DeviceCreate, Some(t.id))
            .await
            .expect("remember");
        assert_broadcast(&mut f.rx, "device_create", "remember_context");

        f.nts.delete(t.id).await.expect("template delete");
        assert_broadcast(&mut f.rx, "device_create", "template delete");

        // --- device soft delete ---------------------------------------------
        let dev = f
            .devices
            .create(device_new(Some("ИНВ-000001")))
            .await
            .expect("device create");
        let dev = match dev {
            trackly_app::dto::device::DeviceSaveOutcome::Created(d) => *d,
            other => panic!("unexpected {other:?}"),
        };
        drain(&mut f.rx);
        f.devices
            .delete_soft(dev.id, dev.version)
            .await
            .expect("device delete");
        assert_broadcast(&mut f.rx, "device_create", "device delete_soft");

        // --- cartridge delete -----------------------------------------------
        let model = f
            .cartridges
            .model_create(CartridgeModelCreateDto {
                brand: "Pantum".into(),
                model: "TL-5120X".into(),
                kind_id: 1,
                color: Some("Чёрный".into()),
                notes: None,
                compatibility: vec![],
            })
            .await
            .expect("model")
            .id;
        let cart = f
            .cartridges
            .create(CartridgeCreateDto {
                model_id: model,
                number_input: number_input("C-0001"),
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("cartridge create")
            .expect_created("cartridge create");
        drain(&mut f.rx);
        f.cartridges
            .delete(cart.id, cart.version)
            .await
            .expect("cartridge delete");
        assert_broadcast(&mut f.rx, "cartridge_create", "cartridge delete");

        // --- act return + act delete ----------------------------------------
        let dev2 = match f
            .devices
            .create(device_new(None))
            .await
            .expect("device for act")
        {
            trackly_app::dto::device::DeviceSaveOutcome::Created(d) => *d,
            other => panic!("unexpected {other:?}"),
        };
        let act = f
            .acts
            .create(
                &Identity::trusted_admin(),
                ActCreateDto {
                    number_input: number_input("42"),
                    giver_name: "Иванов И.И.".into(),
                    receiver_name: "Петров П.П.".into(),
                    place_id: None,
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActItemNewDto {
                        device_id: dev2.id,
                        device_ids: Vec::new(),
                        quantity: 1,
                    }],
                },
            )
            .await
            .expect("act create")
            .expect_created("act create");
        drain(&mut f.rx);
        let item = act.items.first().expect("item");
        f.acts
            .do_return(
                &Identity::trusted_admin(),
                act.id,
                ActReturnDto {
                    bulk_condition: Some("Хорошее".into()),
                    bulk_place_id: None,
                    apply_to_all: true,
                    giver_name: None,
                    receiver_name: None,
                    handover_date_utc: None,
                    items: vec![ActReturnItemDto {
                        act_item_id: item.id,
                        device_id: item.device_id,
                        device_ids: vec![item.device_id],
                        quantity: 1,
                        condition_override: None,
                        place_id_override: None,
                    }],
                },
            )
            .await
            .expect("do_return");
        assert_broadcast(&mut f.rx, "act_create", "do_return");

        let act = f.acts.get(act.id).await.expect("reload act");
        drain(&mut f.rx);
        f.acts
            .delete_soft(act.id, act.version)
            .await
            .expect("act delete");
        assert_broadcast(&mut f.rx, "act_create", "act delete_soft");
    })
    .await
    .expect("budget");
}
