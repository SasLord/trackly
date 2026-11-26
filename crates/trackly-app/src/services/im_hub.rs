//! ImHub — доставка сигналов мессенджера по каналу на пользователя (Phase 43, D-04).
//!
//! Старый `WsEvent`/`ws_broadcast` для IM не расширяется и `is_visible_to` не
//! используется: получатели определяются списком участников беседы, прочитанным
//! в транзакции записи. Сигнал не содержит тела сообщения — только «беседа
//! изменилась до rev N»; содержимое клиент забирает через sync.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;

/// Ёмкость канала одного пользователя; при переполнении подписчик получает Lagged
/// и обязан сделать resync.
pub const IM_CHANNEL_CAPACITY: usize = 256;

/// Сигнал об изменении беседы.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type")]
pub enum ImSignal {
    #[serde(rename = "im_changed", rename_all = "camelCase")]
    Changed { conversation_id: i64, rev: i64 },
}

/// Каналы по пользователям.
#[derive(Debug, Default)]
pub struct ImHub {
    chans: Mutex<HashMap<i64, broadcast::Sender<ImSignal>>>,
}

impl ImHub {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<i64, broadcast::Sender<ImSignal>>> {
        self.chans.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Подписка пользователя; канал создаётся лениво.
    pub fn subscribe(&self, user_id: i64) -> broadcast::Receiver<ImSignal> {
        self.lock()
            .entry(user_id)
            .or_insert_with(|| broadcast::channel(IM_CHANNEL_CAPACITY).0)
            .subscribe()
    }

    /// Публикация только в существующие каналы получателей; отсутствие
    /// подписчиков — не ошибка.
    pub fn publish(&self, recipients: &[i64], signal: ImSignal) {
        let map = self.lock();
        for uid in recipients {
            if let Some(tx) = map.get(uid) {
                let _ = tx.send(signal.clone());
            }
        }
    }

    /// Удаляет каналы без подписчиков.
    pub fn prune(&self) {
        self.lock().retain(|_, tx| tx.receiver_count() > 0);
    }

    #[cfg(test)]
    fn channel_count(&self) -> usize {
        self.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(rev: i64) -> ImSignal {
        ImSignal::Changed {
            conversation_id: 1,
            rev,
        }
    }

    #[tokio::test]
    async fn delivers_only_to_listed_recipients() {
        let hub = ImHub::new();
        let mut a = hub.subscribe(1);
        let mut b = hub.subscribe(2);
        let mut c = hub.subscribe(3);
        hub.publish(&[1, 2], sig(5));
        assert_eq!(a.try_recv().unwrap(), sig(5));
        assert_eq!(b.try_recv().unwrap(), sig(5));
        assert!(matches!(
            c.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn publish_does_not_create_channel() {
        let hub = ImHub::new();
        hub.publish(&[42], sig(1));
        assert_eq!(hub.channel_count(), 0);
    }

    #[tokio::test]
    async fn prune_drops_dead_channels() {
        let hub = ImHub::new();
        let rx = hub.subscribe(1);
        let _keep = hub.subscribe(2);
        drop(rx);
        hub.prune();
        assert_eq!(hub.channel_count(), 1);
    }

    #[tokio::test]
    async fn overflow_yields_lagged() {
        let hub = ImHub::new();
        let mut rx = hub.subscribe(1);
        for i in 0..300 {
            hub.publish(&[1], sig(i));
        }
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Lagged(_))
        ));
    }

    #[test]
    fn signal_serializes_exact_json() {
        let s = serde_json::to_string(&ImSignal::Changed {
            conversation_id: 7,
            rev: 9,
        })
        .unwrap();
        assert_eq!(s, r#"{"type":"im_changed","conversationId":7,"rev":9}"#);
    }
}
