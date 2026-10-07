//! Single owner of the `WsEvent::EntitiesChanged` send rule (Phase 41.7, D-03).
//!
//! Contract for callers:
//! - Send ONLY after the writer closure's `.await?` and OUTSIDE the closure
//!   (D-04): on a rollback no false event exists.
//! - Send unconditionally - NOT under the `if number_changed` guard of the
//!   neighbouring `NumberSpaceChanged` dispatch.
//! - Services wrap this helper in a private method
//!   `broadcast_entities(&self, op, place_ids, device_ids, group_ids)` with a
//!   LITERAL operation label; the layer (1) gate reads service sources by that
//!   method name and label.
//!
//! "Empty" is defined once, here: ALL THREE lists empty -> nothing is sent
//! ("nothing affected"). An event with an empty `place_ids` but non-empty
//! `group_ids`/`device_ids` IS sent, and the client reads it as "reload
//! everything" (D-17).

use std::sync::Arc;

use crate::dto::printer::WsEvent;

/// Normalizes (sort + dedup) the three id lists and broadcasts one
/// `WsEvent::EntitiesChanged`. Best-effort: `None` sender is a silent skip,
/// a channel without subscribers is not an error.
pub(crate) fn send_entities_changed(
    ws_tx: &Option<Arc<tokio::sync::broadcast::Sender<WsEvent>>>,
    op: &'static str,
    mut place_ids: Vec<i64>,
    mut device_ids: Vec<i64>,
    mut group_ids: Vec<i64>,
) {
    for ids in [&mut place_ids, &mut device_ids, &mut group_ids] {
        ids.sort_unstable();
        ids.dedup();
    }
    if place_ids.is_empty() && device_ids.is_empty() && group_ids.is_empty() {
        return;
    }
    let Some(ws_tx) = ws_tx else {
        return;
    };
    tracing::debug!(
        op,
        places = place_ids.len(),
        devices = device_ids.len(),
        groups = group_ids.len(),
        "entities_changed"
    );
    let _ = ws_tx.send(WsEvent::EntitiesChanged {
        place_ids,
        device_ids,
        group_ids,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    fn channel() -> (
        Option<Arc<broadcast::Sender<WsEvent>>>,
        broadcast::Receiver<WsEvent>,
    ) {
        let (tx, rx) = broadcast::channel::<WsEvent>(8);
        (Some(Arc::new(tx)), rx)
    }

    #[test]
    fn non_empty_sends_exactly_one_sorted_event() {
        let (tx, mut rx) = channel();
        send_entities_changed(&tx, "test_op", vec![3, 1, 2], vec![9, 7], vec![5]);
        match rx.try_recv().expect("one event") {
            WsEvent::EntitiesChanged {
                place_ids,
                device_ids,
                group_ids,
            } => {
                assert_eq!(place_ids, vec![1, 2, 3]);
                assert_eq!(device_ids, vec![7, 9]);
                assert_eq!(group_ids, vec![5]);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[test]
    fn duplicates_collapse() {
        let (tx, mut rx) = channel();
        send_entities_changed(&tx, "test_op", vec![2, 2, 1, 1], vec![4, 4], vec![8, 8, 8]);
        match rx.try_recv().expect("one event") {
            WsEvent::EntitiesChanged {
                place_ids,
                device_ids,
                group_ids,
            } => {
                assert_eq!(place_ids, vec![1, 2]);
                assert_eq!(device_ids, vec![4]);
                assert_eq!(group_ids, vec![8]);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn all_empty_sends_nothing() {
        let (tx, mut rx) = channel();
        send_entities_changed(&tx, "test_op", vec![], vec![], vec![]);
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[test]
    fn empty_places_with_groups_still_sends() {
        let (tx, mut rx) = channel();
        send_entities_changed(&tx, "test_op", vec![], vec![], vec![4]);
        assert!(matches!(rx.try_recv(), Ok(WsEvent::EntitiesChanged { .. })));
    }

    #[test]
    fn none_sender_does_not_panic() {
        send_entities_changed(&None, "test_op", vec![1], vec![], vec![]);
    }

    #[test]
    fn channel_without_subscribers_does_not_panic() {
        let (tx, rx) = channel();
        drop(rx);
        send_entities_changed(&tx, "test_op", vec![1], vec![2], vec![3]);
    }
}
