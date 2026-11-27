//! LinkHub — разрыв WebSocket-соединений Link по устройству или пользователю
//! (Phase 44, D-14). Хаб не знает про БД: сервис отзыва зовёт его после коммита.

use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

#[derive(Debug)]
struct Entry {
    device_id: String,
    user_id: i64,
    token: CancellationToken,
}

/// Реестр живых соединений Link.
#[derive(Debug, Default)]
pub struct LinkHub {
    entries: Mutex<Vec<Entry>>,
}

impl LinkHub {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Entry>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Регистрирует соединение; WS-обработчик ждёт отмены возвращённого токена.
    pub fn register(&self, device_id: &str, user_id: i64) -> CancellationToken {
        let token = CancellationToken::new();
        let mut g = self.lock();
        g.retain(|e| !e.token.is_cancelled());
        g.push(Entry {
            device_id: device_id.to_string(),
            user_id,
            token: token.clone(),
        });
        token
    }

    /// Разрывает все соединения устройства; возвращает их число.
    pub fn revoke_device(&self, device_id: &str) -> usize {
        self.cancel_where(|e| e.device_id == device_id)
    }

    /// Разрывает все соединения пользователя; возвращает их число.
    pub fn revoke_user(&self, user_id: i64) -> usize {
        self.cancel_where(|e| e.user_id == user_id)
    }

    fn cancel_where(&self, pred: impl Fn(&Entry) -> bool) -> usize {
        let mut g = self.lock();
        let mut n = 0;
        for e in g.iter().filter(|e| pred(e)) {
            e.token.cancel();
            n += 1;
        }
        g.retain(|e| !e.token.is_cancelled());
        n
    }

    /// Удаляет записи с отменёнными токенами (соединение закрылось само).
    pub fn prune(&self) {
        self.lock().retain(|e| !e.token.is_cancelled());
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_device_cancels_only_that_device() {
        let hub = LinkHub::new();
        let a1 = hub.register("dev-a", 1);
        let a2 = hub.register("dev-a", 2);
        let b = hub.register("dev-b", 1);
        assert_eq!(hub.revoke_device("dev-a"), 2);
        assert!(a1.is_cancelled() && a2.is_cancelled());
        assert!(!b.is_cancelled());
    }

    #[test]
    fn revoke_user_cancels_all_user_devices() {
        let hub = LinkHub::new();
        let a = hub.register("dev-a", 7);
        let b = hub.register("dev-b", 7);
        let c = hub.register("dev-c", 8);
        assert_eq!(hub.revoke_user(7), 2);
        assert!(a.is_cancelled() && b.is_cancelled());
        assert!(!c.is_cancelled());
    }

    #[test]
    fn revoke_prunes_and_repeat_is_harmless() {
        let hub = LinkHub::new();
        hub.register("dev-a", 1);
        assert_eq!(hub.revoke_device("dev-a"), 1);
        assert_eq!(hub.len(), 0);
        assert_eq!(hub.revoke_device("dev-a"), 0);
        hub.prune();
    }

    #[test]
    fn new_connection_survives_old_revoke() {
        let hub = LinkHub::new();
        let old = hub.register("dev-a", 1);
        hub.revoke_device("dev-a");
        let fresh = hub.register("dev-a", 1);
        assert!(old.is_cancelled());
        assert!(!fresh.is_cancelled());
        assert_eq!(hub.len(), 1);
    }

    #[test]
    fn poisoned_mutex_does_not_break_hub() {
        let hub = std::sync::Arc::new(LinkHub::new());
        let h2 = hub.clone();
        let _ = std::thread::spawn(move || {
            let _g = h2.entries.lock().unwrap();
            panic!("poison");
        })
        .join();
        let t = hub.register("dev-a", 1);
        assert_eq!(hub.revoke_device("dev-a"), 1);
        assert!(t.is_cancelled());
    }
}
