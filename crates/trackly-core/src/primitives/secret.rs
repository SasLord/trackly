//! `Secret<T>` — обёртка над чувствительными значениями.
//!
//! Контракт (D-Security V7):
//! - **Не реализует** `Debug`/`Display` для типа `T` напрямую — ручная реализация
//!   `Debug` выводит литерал `"***"`, чтобы случайный `format!("{:?}", config)`
//!   никогда не утёк пароль в логи.
//! - **Не реализует** `Serialize`/`Deserialize` — секреты должны явно
//!   присутствовать в DTO; авто-сериализация = утечка через JSON API.
//!   Compile-time гарантия в `tests/secret_zeroize.rs` через `static_assertions`.
//! - **Drop** вызывает `zeroize()` на внутреннем значении — после drop'а
//!   память обнулена (best-effort: оптимизатор не имеет права выбросить вызов
//!   из-за `#[inline(never)]` внутри crate `zeroize`).
//!
//! `T: Zeroize + Clone` потому что:
//! - `Zeroize` нужен для безопасного освобождения.
//! - `Clone` нужен для удобства callers (например, скопировать пароль из конфига
//!   в новый `Secret` для bind в LDAP без манёвров с lifetimes).

use std::fmt;
use zeroize::Zeroize;

/// Newtype-обёртка, гарантирующая zeroize-on-drop и невозможность утечки через
/// `Debug` / `Serialize`. Используется для паролей, токенов, ключей.
pub struct Secret<T: Zeroize + Clone>(T);

impl<T: Zeroize + Clone> Secret<T> {
    /// Заворачивает значение в `Secret`. После вызова владение переходит к `Secret`,
    /// и единственный способ получить ссылку на значение — [`Secret::expose`].
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Возвращает &-ссылку на внутреннее значение. Имя `expose` намеренно
    /// длинное — на code-review должно быть очевидно, что здесь начинается
    /// зона повышенного риска (например, передача в LDAP bind).
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize + Clone> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl<T: Zeroize + Clone> Drop for Secret<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

// ВНИМАНИЕ: НЕ ДОБАВЛЯТЬ derive Serialize / Deserialize / Display / Clone здесь.
// Compile-time проверка в `crates/trackly-core/tests/secret_zeroize.rs`
// (`static_assertions::assert_not_impl_all!`).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_does_not_leak_value() {
        let s = Secret::new(String::from("hunter2"));
        let dbg = format!("{s:?}");
        assert_eq!(dbg, "***");
        assert!(!dbg.contains("hunter2"));
    }

    #[test]
    fn expose_returns_original_value() {
        let s = Secret::new(String::from("password123"));
        assert_eq!(s.expose(), "password123");
    }

    #[test]
    fn debug_inside_vec_hides_all_values() {
        let v: Vec<Secret<String>> = (0..5)
            .map(|_| Secret::new(String::from("hunter2")))
            .collect();
        let dbg = format!("{v:?}");
        assert!(!dbg.contains("hunter2"));
        assert_eq!(dbg.matches("***").count(), 5);
    }
}
