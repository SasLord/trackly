//! Phase 41.2 plan 16: страж ПОРЯДКА «план групп и подтверждение стоят выше первой
//! записи» в `crates/trackly-app/src/services/act_service.rs`.
//!
//! Почему страж по исходнику, а не тест поведения. Транзакция writer'а, сброшенная
//! ранним `return Ok(ActWriteStep::NeedsGroupRelease(..))`, откатывает всё, что успело
//! записаться ВЫШЕ. Поэтому ни один счётчик строк не отличает «подтверждение до
//! записи» от «подтверждение после записи, которую откатили»: поведение у обоих
//! порядков одинаковое, расходится только исходник. То же для порядка шагов в ветках
//! `removed` правки и правки возврата: все чтения и записи лежат в одной транзакции.
//! (Порядок ОТКАТА акта, напротив, держат тесты поведения: мутанты M4, M4b, M8 плана
//! 41.2-13; здесь он не дублируется.)
//!
//! Что охраняется:
//!   (a) `plan_in_tx` / `plan_for_return_in_tx` стоит выше ЛЮБОЙ записи в `create`,
//!       `update`, `do_return` (первая запись — вызов из `WRITE_MARKERS`);
//!   (b) ранний возврат `ActWriteStep::NeedsGroupRelease(` в `create` и `update` стоит
//!       выше той же первой записи;
//!   (c) `apply_plan_in_tx` стоит ниже `plan_in_tx` в `create` и `update`;
//!   (d) `do_return`: `plan_for_return_in_tx` < `apply_return_plan_in_tx` <
//!       `update_full_in_tx`; `update_return`: то же попарно для каждой из двух стадий
//!       (добавленные, сохранённые с правкой);
//!   (e) ветки `removed` правки и правки возврата: читатель прежнего эффекта release
//!       идёт ПЕРВЫМ (W-7, V-4), `revert_move_effect_in_tx`, `anchor_restore_conflict_in_tx`
//!       и `release_effect_for_removed_in_tx` выше `restore_from_snapshot_in_tx`,
//!       `restore_membership_in_tx` ниже;
//!   (e2) после единственного `release_if_locked_device_in_tx` в теле нет ни повторного
//!       чтения эффектов акта, ни второго `prior_release_effect_id_in_tx`: повторное чтение
//!       подобрало бы только что записанный след `Record`, эффект удалился бы, откат не
//!       вернул бы устройство в группу;
//!   (f) `undo_device_mutations_for_act`: сначала группы, затем снимки, затем возврат
//!       членства, затем удаление эффектов;
//!   (g) `update_return`: план добавленных читается ПОСЛЕ ветки `removed` (так задумано
//!       планом 41.2-11: снимок группы должен видеть группы, уже вернувшиеся на старое
//!       место). ВАЖНО: на правке возврата «план выше первой записи» НЕ держится и держаться
//!       не должен: ветка `removed` пишет раньше, а у возврата нет исхода подтверждения;
//!   (h) `enum ActWriteStep` объявлен на уровне модуля, вне тел `create` и `update`.
//!
//! Файл-цель подключён `include_str!`: переименование или удаление файла — ошибка
//! КОМПИЛЯЦИИ, а не тихий пропуск. Переименованная функция или якорь — красный тест
//! (сигнатура и кратность якорей проверяются), а не зелёный.
//!
//! Самопроверка на копии исходника В ПАМЯТИ (`_mutation`, `_selftest`): каждый мутант
//! обязан краснеть тем стражем, который он нацелен снять; чистая копия зелёная.
//! Перестановки — ОБМЕН МЕСТАМИ двух строк с n-ми вхождениями якорей внутри диапазона
//! тела (а не replace по файлу): счёт вхождений не меняется, меняется только порядок.

const SRC: &str = include_str!("../src/services/act_service.rs");

// ---------------------------------------------------------------------------
// Разбор исходника
// ---------------------------------------------------------------------------

/// Код строки без хвоста `// ...` (в т.ч. строка-комментарий целиком).
fn code_of(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Позиции вхождений `ident` в `text`, у которых слева НЕТ символа идентификатора:
/// `plan_in_tx(` не есть подстрока `apply_plan_in_tx(` (W-4).
fn call_positions(text: &str, ident: &str) -> Vec<usize> {
    text.match_indices(ident)
        .filter(|(pos, _)| {
            text[..*pos]
                .chars()
                .next_back()
                .is_none_or(|c| !is_ident_char(c))
        })
        .map(|(pos, _)| pos)
        .collect()
}

fn is_fn_header(trimmed: &str) -> bool {
    [
        "pub async fn ",
        "async fn ",
        "pub fn ",
        "pub(crate) fn ",
        "fn ",
    ]
    .iter()
    .any(|p| trimmed.starts_with(p))
}

/// Диапазон байтов тела функции в ИСХОДНОМ тексте: от строки с сигнатурой до ближайшей
/// следующей строки заголовка функции с тем же отступом. Сигнатура обязана встречаться
/// в файле РОВНО один раз.
fn fn_range(src: &str, signature: &str) -> Result<(usize, usize), String> {
    let hits: Vec<usize> = src.match_indices(signature).map(|(p, _)| p).collect();
    if hits.len() != 1 {
        return Err(format!(
            "сигнатура `{signature}` встречается {} раз(а), нужно ровно 1",
            hits.len()
        ));
    }
    let start_line_begin = src[..hits[0]].rfind('\n').map_or(0, |p| p + 1);
    let header_line = src[start_line_begin..].lines().next().unwrap_or("");
    let header_indent = header_line.len() - header_line.trim_start().len();

    let mut offset = start_line_begin;
    let mut first = true;
    for line in src[start_line_begin..].split_inclusive('\n') {
        if !first {
            let trimmed = line.trim_start();
            let ind = line.len() - trimmed.len();
            if ind == header_indent && is_fn_header(trimmed) {
                return Ok((start_line_begin, offset));
            }
        }
        first = false;
        offset += line.len();
    }
    Ok((start_line_begin, src.len()))
}

/// Тело функции без комментариев (структура строк сохраняется).
fn body_of(src: &str, signature: &str) -> String {
    let (a, b) = fn_range(src, signature).unwrap_or_else(|e| panic!("{e}"));
    src[a..b]
        .split_inclusive('\n')
        .map(|l| {
            let code = code_of(l);
            if l.ends_with('\n') {
                format!("{code}\n")
            } else {
                code.to_string()
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Таблица функций и якорей
// ---------------------------------------------------------------------------

const SIG_CREATE: &str = "pub async fn create(";
const SIG_UPDATE: &str = "pub async fn update(";
const SIG_DO_RETURN: &str = "pub async fn do_return(";
const SIG_UPDATE_RETURN: &str = "pub async fn update_return(";
const SIG_UNDO: &str = "fn undo_device_mutations_for_act(";

/// Все четыре пути записи, на которых охраняется порядок «план -> запись».
const WRITE_PATHS: [&str; 4] = [SIG_CREATE, SIG_UPDATE, SIG_DO_RETURN, SIG_UPDATE_RETURN];

/// Первая запись тела: вызовы, после которых транзакция уже не «пустая». Раннее чтение
/// и вычисление плана к ним не относятся.
const WRITE_MARKERS: [&str; 10] = [
    "insert_act_in_tx(",
    "insert_act_item_in_tx(",
    "apply_plan_in_tx(",
    "apply_return_plan_in_tx(",
    "update_full_in_tx(",
    "update_status_and_place_in_tx(",
    "release_device_in_tx(",
    "clone_device_in_tx(",
    "update_act_header_in_tx(",
    "tx.execute(",
];

/// Якорь и требуемая кратность в теле: `Exactly(n)` либо `AtLeast(n)`.
#[derive(Clone, Copy)]
enum Count {
    Exactly(usize),
    AtLeast(usize),
}

/// (сигнатура, якоря). Кратность задана явно (W-9): якоря, которых в теле по одному,
/// проверяются на «ровно один», остальные — на минимум и берутся по ПЕРВОМУ/n-му
/// вхождению.
fn anchor_table() -> Vec<(&'static str, Vec<(&'static str, Count)>)> {
    use Count::{AtLeast, Exactly};
    vec![
        (
            SIG_CREATE,
            vec![
                ("plan_in_tx(", Exactly(1)),
                ("insert_act_in_tx(", Exactly(1)),
                ("apply_plan_in_tx(", Exactly(1)),
                ("ActWriteStep::NeedsGroupRelease(", AtLeast(2)),
            ],
        ),
        (
            SIG_UPDATE,
            vec![
                ("plan_in_tx(", Exactly(1)),
                ("apply_plan_in_tx(", Exactly(1)),
                ("revert_move_effect_in_tx(", Exactly(1)),
                ("prior_release_effect_id_in_tx(", Exactly(1)),
                ("release_effect_for_removed_in_tx(", Exactly(1)),
                ("anchor_restore_conflict_in_tx(", Exactly(1)),
                ("restore_from_snapshot_in_tx(", Exactly(1)),
                ("release_if_locked_device_in_tx(", Exactly(1)),
                ("restore_membership_in_tx(", Exactly(1)),
                ("ActWriteStep::NeedsGroupRelease(", AtLeast(2)),
            ],
        ),
        (
            SIG_DO_RETURN,
            vec![
                ("plan_for_return_in_tx(", Exactly(1)),
                ("insert_act_in_tx(", Exactly(1)),
                ("apply_return_plan_in_tx(", Exactly(1)),
                ("update_full_in_tx(", Exactly(1)),
            ],
        ),
        (
            SIG_UPDATE_RETURN,
            vec![
                ("plan_for_return_in_tx(", Exactly(2)),
                ("apply_return_plan_in_tx(", Exactly(2)),
                ("update_full_in_tx(", Exactly(2)),
                ("revert_move_effect_in_tx(", Exactly(1)),
                ("prior_release_effect_id_in_tx(", Exactly(1)),
                ("release_effect_for_removed_in_tx(", Exactly(1)),
                ("anchor_restore_conflict_in_tx(", Exactly(1)),
                ("restore_from_snapshot_in_tx(", Exactly(1)),
                ("release_if_locked_device_in_tx(", Exactly(1)),
                ("restore_membership_in_tx(", Exactly(1)),
                ("delete_act_group_effect_in_tx(", AtLeast(1)),
            ],
        ),
        (
            SIG_UNDO,
            vec![
                ("revert_group_moves_in_tx(", Exactly(1)),
                ("anchor_restore_conflict_in_tx(", Exactly(1)),
                ("restore_from_snapshot_in_tx(", Exactly(1)),
                ("restore_released_members_in_tx(", Exactly(1)),
                ("delete_act_group_effects_in_tx(", Exactly(1)),
            ],
        ),
    ]
}

fn pos(body: &str, ident: &str, nth: usize) -> Option<usize> {
    call_positions(body, ident).get(nth).copied()
}

fn last_pos(body: &str, ident: &str) -> Option<usize> {
    call_positions(body, ident).last().copied()
}

/// Позиция первой записи в теле: минимум по `WRITE_MARKERS` и по тексту
/// `for &dev_id in &added` (в `update` это цикл статус-проверки перед переносом).
fn first_write(body: &str, include_added_loop: bool) -> Option<usize> {
    let mut best: Option<usize> = None;
    for m in WRITE_MARKERS {
        if let Some(p) = pos(body, m, 0) {
            best = Some(best.map_or(p, |b| b.min(p)));
        }
    }
    if include_added_loop {
        if let Some(p) = body.find("for &dev_id in &added") {
            best = Some(best.map_or(p, |b| b.min(p)));
        }
    }
    best
}

/// Все нарушения исходника: пустой вектор = зелёный. Строки начинаются с метки стража.
fn defects(src: &str) -> Vec<String> {
    let mut out = structure_defects(src);
    if out.is_empty() {
        out = order_defects(src);
    }
    out
}

/// Сигнатуры и кратность якорей (мёртвый страж из-за переименования был бы вакуумно
/// зелёным).
fn structure_defects(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for (sig, anchors) in anchor_table() {
        if let Err(e) = fn_range(src, sig) {
            out.push(format!("(cnt) {e}"));
            continue;
        }
        let body = body_of(src, sig);
        for (anchor, want) in anchors {
            let n = call_positions(&body, anchor).len();
            let ok = match want {
                Count::Exactly(k) => n == k,
                Count::AtLeast(k) => n >= k,
            };
            if !ok {
                out.push(format!(
                    "(cnt) {sig} якорь `{anchor}`: {n} вхождений, ожидалось {}",
                    match want {
                        Count::Exactly(k) => format!("ровно {k}"),
                        Count::AtLeast(k) => format!("не меньше {k}"),
                    }
                ));
            }
        }
    }
    out
}

/// Порядок вызовов. Предполагает, что `structure_defects` пуст (якоря на месте).
fn order_defects(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let create = body_of(src, SIG_CREATE);
    let update = body_of(src, SIG_UPDATE);
    let do_return = body_of(src, SIG_DO_RETURN);
    let update_return = body_of(src, SIG_UPDATE_RETURN);
    let undo = body_of(src, SIG_UNDO);

    // (a) / (b) / (c): create и update.
    for (label, body, with_loop) in [("create", &create, false), ("update", &update, true)] {
        let plan = pos(body, "plan_in_tx(", 0).expect("plan_in_tx");
        let write = first_write(body, with_loop).expect("первая запись");
        if plan >= write {
            out.push(format!(
                "(a) {label}: plan_in_tx( ({plan}) не выше первой записи ({write})"
            ));
        }
        let early = pos(body, "ActWriteStep::NeedsGroupRelease(", 0).expect("ранний возврат");
        if early >= write {
            out.push(format!(
                "(b) {label}: ранний возврат NeedsGroupRelease ({early}) не выше первой записи ({write})"
            ));
        }
        if early <= plan {
            out.push(format!(
                "(b) {label}: ранний возврат ({early}) стоит не ниже плана ({plan})"
            ));
        }
        let apply = pos(body, "apply_plan_in_tx(", 0).expect("apply_plan_in_tx");
        if apply <= plan {
            out.push(format!(
                "(c) {label}: apply_plan_in_tx( ({apply}) не ниже plan_in_tx( ({plan})"
            ));
        }
    }

    // (a) / (d): do_return.
    {
        let plan = pos(&do_return, "plan_for_return_in_tx(", 0).expect("plan");
        let write = first_write(&do_return, false).expect("запись");
        if plan >= write {
            out.push(format!(
                "(a) do_return: plan_for_return_in_tx( ({plan}) не выше первой записи ({write})"
            ));
        }
        let apply = pos(&do_return, "apply_return_plan_in_tx(", 0).expect("apply");
        let upd = pos(&do_return, "update_full_in_tx(", 0).expect("update_full");
        if apply <= plan {
            out.push(format!(
                "(d) do_return: apply_return_plan_in_tx( ({apply}) не ниже плана ({plan})"
            ));
        }
        if apply >= upd {
            out.push(format!(
                "(d) do_return: apply_return_plan_in_tx( ({apply}) не выше update_full_in_tx( ({upd})"
            ));
        }
    }

    // (d): update_return, обе стадии попарно.
    for stage in 0..2 {
        let plan = pos(&update_return, "plan_for_return_in_tx(", stage).expect("plan");
        let apply = pos(&update_return, "apply_return_plan_in_tx(", stage).expect("apply");
        let upd = pos(&update_return, "update_full_in_tx(", stage).expect("update_full");
        if apply <= plan {
            out.push(format!(
                "(d) update_return стадия {stage}: apply_return_plan_in_tx( ({apply}) не ниже плана ({plan})"
            ));
        }
        if apply >= upd {
            out.push(format!(
                "(d) update_return стадия {stage}: apply_return_plan_in_tx( ({apply}) не выше update_full_in_tx( ({upd})"
            ));
        }
    }

    // (g): план добавленных в update_return читается ПОСЛЕ ветки removed.
    {
        let removed_end =
            last_pos(&update_return, "delete_act_group_effect_in_tx(").expect("delete");
        let plan = pos(&update_return, "plan_for_return_in_tx(", 0).expect("plan");
        if plan <= removed_end {
            out.push(format!(
                "(g) update_return: план добавленных ({plan}) читается не ниже ветки removed ({removed_end})"
            ));
        }
    }

    // (e) / (e2): ветки removed правки и правки возврата.
    for (label, body) in [("update", &update), ("update_return", &update_return)] {
        let revert = pos(body, "revert_move_effect_in_tx(", 0).expect("revert");
        let prior = pos(body, "prior_release_effect_id_in_tx(", 0).expect("prior");
        let mode = pos(body, "release_effect_for_removed_in_tx(", 0).expect("mode");
        let conflict = pos(body, "anchor_restore_conflict_in_tx(", 0).expect("conflict");
        let snap = pos(body, "restore_from_snapshot_in_tx(", 0).expect("snap");
        let release = pos(body, "release_if_locked_device_in_tx(", 0).expect("release");
        let member = pos(body, "restore_membership_in_tx(", 0).expect("member");
        if prior >= mode {
            out.push(format!(
                "(e) {label}: prior_release_effect_id_in_tx( ({prior}) не выше release_effect_for_removed_in_tx( ({mode})"
            ));
        }
        if prior >= snap {
            out.push(format!(
                "(e) {label}: prior_release_effect_id_in_tx( ({prior}) не выше restore_from_snapshot_in_tx( ({snap})"
            ));
        }
        if revert >= snap {
            out.push(format!(
                "(e) {label}: revert_move_effect_in_tx( ({revert}) не выше restore_from_snapshot_in_tx( ({snap})"
            ));
        }
        if conflict >= snap {
            out.push(format!(
                "(e) {label}: anchor_restore_conflict_in_tx( ({conflict}) не выше restore_from_snapshot_in_tx( ({snap})"
            ));
        }
        if mode >= snap {
            out.push(format!(
                "(e) {label}: release_effect_for_removed_in_tx( ({mode}) не выше restore_from_snapshot_in_tx( ({snap})"
            ));
        }
        if member <= snap {
            out.push(format!(
                "(e) {label}: restore_membership_in_tx( ({member}) не ниже restore_from_snapshot_in_tx( ({snap})"
            ));
        }
        let tail = &body[release..];
        for forbidden in [
            "list_act_group_effects_in_tx(",
            "prior_release_effect_id_in_tx(",
        ] {
            if !call_positions(tail, forbidden).is_empty() {
                out.push(format!(
                    "(e2) {label}: после release_if_locked_device_in_tx( есть {forbidden}"
                ));
            }
        }
    }

    // (f): откат акта.
    {
        let order = [
            "revert_group_moves_in_tx(",
            "anchor_restore_conflict_in_tx(",
            "restore_from_snapshot_in_tx(",
            "restore_released_members_in_tx(",
            "delete_act_group_effects_in_tx(",
        ];
        let ps: Vec<usize> = order
            .iter()
            .map(|a| pos(&undo, a, 0).expect("якорь undo"))
            .collect();
        for w in 0..order.len() - 1 {
            if ps[w] >= ps[w + 1] {
                out.push(format!(
                    "(f) undo: {} ({}) не выше {} ({})",
                    order[w],
                    ps[w],
                    order[w + 1],
                    ps[w + 1]
                ));
            }
        }
    }

    // (h): enum ActWriteStep на уровне модуля, вне тел create и update.
    {
        let decls: Vec<usize> = src
            .match_indices('\n')
            .map(|(p, _)| p + 1)
            .chain(std::iter::once(0))
            .filter(|&p| {
                let rest = &src[p..];
                rest.starts_with("enum ActWriteStep")
                    || rest.starts_with("pub(crate) enum ActWriteStep")
            })
            .collect();
        if decls.len() != 1 {
            out.push(format!(
                "(h) enum ActWriteStep объявлен {} раз на уровне модуля, нужно ровно 1",
                decls.len()
            ));
        } else {
            for sig in [SIG_CREATE, SIG_UPDATE] {
                let (a, b) = fn_range(src, sig).expect("range");
                if decls[0] >= a && decls[0] < b {
                    out.push(format!("(h) enum ActWriteStep объявлен внутри тела {sig}"));
                }
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Мутации на копии исходника в памяти
// ---------------------------------------------------------------------------

/// Индекс (в `split_inclusive('\n')`) строки с n-м вхождением якоря внутри диапазона
/// тела `sig`. `nth == usize::MAX` — последнее вхождение.
fn line_of_anchor(lines: &[String], range: (usize, usize), anchor: &str, nth: usize) -> usize {
    let mut offset = 0;
    let mut hits: Vec<usize> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let begin = offset;
        offset += l.len();
        if begin < range.0 || begin >= range.1 {
            continue;
        }
        if !call_positions(code_of(l), anchor).is_empty() {
            hits.push(i);
        }
    }
    let idx = if nth == usize::MAX {
        hits.len().checked_sub(1)
    } else {
        Some(nth)
    };
    *idx.and_then(|i| hits.get(i)).unwrap_or_else(|| {
        panic!(
            "якорь `{anchor}` (n={nth}) не найден в {range:?}; найдено {}",
            hits.len()
        )
    })
}

/// ОБМЕН МЕСТАМИ двух строк с (n-м) вхождениями якорей внутри тела `sig`.
fn swap_lines(src: &str, sig: &str, a: (&str, usize), b: (&str, usize)) -> String {
    let range = fn_range(src, sig).expect("range");
    let mut lines: Vec<String> = src.split_inclusive('\n').map(str::to_string).collect();
    let ia = line_of_anchor(&lines, range, a.0, a.1);
    let ib = line_of_anchor(&lines, range, b.0, b.1);
    assert_ne!(
        ia, ib,
        "якоря {} и {} на одной строке — обмен невозможен",
        a.0, b.0
    );
    lines.swap(ia, ib);
    lines.concat()
}

/// Вставка строки сразу после строки с n-м вхождением якоря внутри тела `sig`.
fn insert_after(src: &str, sig: &str, anchor: (&str, usize), new_line: &str) -> String {
    let range = fn_range(src, sig).expect("range");
    let mut lines: Vec<String> = src.split_inclusive('\n').map(str::to_string).collect();
    let i = line_of_anchor(&lines, range, anchor.0, anchor.1);
    lines.insert(i + 1, format!("{new_line}\n"));
    lines.concat()
}

/// Мутант-УДАЛЕНИЕ: первое вхождение якоря в теле `sig` выключается суффиксом.
fn disable_first(src: &str, sig: &str, anchor: &str) -> String {
    let (a, b) = fn_range(src, sig).expect("range");
    let body = &src[a..b];
    let at = *call_positions(body, anchor)
        .first()
        .unwrap_or_else(|| panic!("якорь `{anchor}` не найден"));
    // суффикс идентификатора перед `(`
    let ident_end = at + anchor.len() - 1;
    let mut out = String::with_capacity(src.len() + 3);
    out.push_str(&src[..a + ident_end]);
    out.push_str("Off");
    out.push_str(&src[a + ident_end..]);
    out
}

fn counts(src: &str, sig: &str, anchors: &[&str]) -> Vec<usize> {
    let body = body_of(src, sig);
    anchors
        .iter()
        .map(|a| call_positions(&body, a).len())
        .collect()
}

fn has(defects: &[String], label: &str) -> bool {
    defects.iter().any(|d| d.starts_with(label))
}

// ---------------------------------------------------------------------------
// Тесты
// ---------------------------------------------------------------------------

#[test]
fn plan_order_guard_clean_source_is_green() {
    let d = defects(SRC);
    assert!(
        d.is_empty(),
        "порядок «план -> запись» нарушен:\n{}",
        d.join("\n")
    );
}

#[test]
fn plan_order_guard_covers_all_four_write_paths() {
    // Страж не вакуумен: все четыре пути записи присутствуют в таблице якорей, и их
    // сигнатуры находятся в файле ровно по одному разу.
    let table = anchor_table();
    for sig in WRITE_PATHS {
        assert!(
            table.iter().any(|(s, a)| *s == sig && !a.is_empty()),
            "путь {sig} не охвачен таблицей якорей"
        );
        fn_range(SRC, sig).unwrap_or_else(|e| panic!("{e}"));
    }
    fn_range(SRC, SIG_UNDO).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(WRITE_PATHS.len(), 4);
}

#[test]
fn plan_order_guard_selftest_identifier_boundary() {
    // W-4: `plan_in_tx(` не есть подстрока `apply_plan_in_tx(`.
    let fixture = "apply_plan_in_tx(x); plan_in_tx(y);";
    assert_eq!(call_positions(fixture, "plan_in_tx(").len(), 1);
    assert_eq!(call_positions(fixture, "apply_plan_in_tx(").len(), 1);
    assert_eq!(
        call_positions(fixture, "plan_in_tx(")[0],
        fixture.find("; plan").unwrap() + 2
    );
    // `plan_for_return_in_tx(` не содержит `plan_in_tx(`.
    assert!(call_positions("plan_for_return_in_tx(a);", "plan_in_tx(").is_empty());
    // Хвост-комментарий и строка-комментарий не дают вхождений в body_of.
    let src = "    pub async fn a() {\n        // plan_in_tx(x)\n        let _ = 1; // plan_in_tx(y)\n        plan_in_tx(z);\n    }\n    fn b() {}\n";
    let body = body_of(src, "pub async fn a(");
    assert_eq!(call_positions(&body, "plan_in_tx(").len(), 1);
    assert!(
        !body.contains("fn b"),
        "тело заканчивается на следующей функции"
    );
}

#[test]
fn plan_order_guard_selftest_missing_or_renamed_anchor_is_an_error() {
    // Переименованная функция и пропавший якорь — ошибка, а не пропуск.
    let renamed = SRC.replace("pub async fn do_return(", "pub async fn do_return_x(");
    assert!(has(&defects(&renamed), "(cnt)"));
    let lost = disable_first(SRC, SIG_UPDATE_RETURN, "prior_release_effect_id_in_tx(");
    assert!(has(&defects(&lost), "(cnt)"));
    // Два объявления enum — ошибка (h) тоже поймана.
    let dup = format!("{SRC}\nenum ActWriteStep {{ A }}\n");
    assert!(has(&defects(&dup), "(h)"));
}

// (1) create: plan_in_tx <-> insert_act_in_tx.
#[test]
fn plan_order_guard_mutant_1_create_plan_after_insert() {
    let anchors = ["plan_in_tx(", "insert_act_in_tx(", "apply_plan_in_tx("];
    let m = swap_lines(
        SRC,
        SIG_CREATE,
        ("plan_in_tx(", 0),
        ("insert_act_in_tx(", 0),
    );
    assert_ne!(m, SRC);
    assert_eq!(
        counts(&m, SIG_CREATE, &anchors),
        counts(SRC, SIG_CREATE, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(a) create"), "{d:?}");
    assert!(!has(&d, "(cnt)"), "перестановка не меняет счёт: {d:?}");
}

// (2) undo: restore_released_members_in_tx <-> restore_from_snapshot_in_tx.
#[test]
fn plan_order_guard_mutant_2_undo_members_before_snapshots() {
    let anchors = [
        "restore_released_members_in_tx(",
        "restore_from_snapshot_in_tx(",
    ];
    let m = swap_lines(
        SRC,
        SIG_UNDO,
        ("restore_released_members_in_tx(", 0),
        ("restore_from_snapshot_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_UNDO, &anchors),
        counts(SRC, SIG_UNDO, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(f) undo"), "{d:?}");
    assert!(!has(&d, "(cnt)"), "{d:?}");
}

// (3) update: restore_membership_in_tx <-> restore_from_snapshot_in_tx.
#[test]
fn plan_order_guard_mutant_3_update_membership_before_snapshot() {
    let anchors = ["restore_membership_in_tx(", "restore_from_snapshot_in_tx("];
    let m = swap_lines(
        SRC,
        SIG_UPDATE,
        ("restore_membership_in_tx(", 0),
        ("restore_from_snapshot_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_UPDATE, &anchors),
        counts(SRC, SIG_UPDATE, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(e) update"), "{d:?}");
    assert!(!has(&d, "(cnt)"), "{d:?}");
}

// (4) do_return: apply_return_plan_in_tx <-> update_full_in_tx.
#[test]
fn plan_order_guard_mutant_4_do_return_apply_after_device_write() {
    let anchors = ["apply_return_plan_in_tx(", "update_full_in_tx("];
    let m = swap_lines(
        SRC,
        SIG_DO_RETURN,
        ("apply_return_plan_in_tx(", 0),
        ("update_full_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_DO_RETURN, &anchors),
        counts(SRC, SIG_DO_RETURN, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(d) do_return"), "{d:?}");
    assert!(!has(&d, "(cnt)"), "{d:?}");
}

// (5) update_return: anchor_restore_conflict_in_tx <-> restore_from_snapshot_in_tx.
#[test]
fn plan_order_guard_mutant_5_update_return_anchor_guard_after_restore() {
    let anchors = [
        "anchor_restore_conflict_in_tx(",
        "restore_from_snapshot_in_tx(",
    ];
    let m = swap_lines(
        SRC,
        SIG_UPDATE_RETURN,
        ("anchor_restore_conflict_in_tx(", 0),
        ("restore_from_snapshot_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_UPDATE_RETURN, &anchors),
        counts(SRC, SIG_UPDATE_RETURN, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(e) update_return"), "{d:?}");
    assert!(!has(&d, "(cnt)"), "{d:?}");
}

// (6) удаление: первое NeedsGroupRelease( в create и в update выключается.
#[test]
fn plan_order_guard_mutant_6_early_return_disabled() {
    for (sig, label) in [(SIG_CREATE, "(b) create"), (SIG_UPDATE, "(b) update")] {
        let anchors = ["ActWriteStep::NeedsGroupRelease("];
        let m = disable_first(SRC, sig, "ActWriteStep::NeedsGroupRelease(");
        assert_ne!(m, SRC);
        assert_eq!(
            counts(&m, sig, &anchors)[0] + 1,
            counts(SRC, sig, &anchors)[0],
            "счёт якоря уменьшается ровно на 1"
        );
        // Счётный страж краснеет (минимум 2)...
        assert!(has(&defects(&m), "(cnt)"), "{label}: {:?}", defects(&m));
        // ...и порядок ловится независимо от счёта: первым в теле остаётся ветка match
        // ПОСЛЕ writer, позже записи.
        let d = order_defects(&m);
        assert!(has(&d, label), "{label}: {d:?}");
    }
}

// (7) update: prior_release_effect_id_in_tx <-> release_effect_for_removed_in_tx.
#[test]
fn plan_order_guard_mutant_7_update_prior_release_read_late() {
    let anchors = [
        "prior_release_effect_id_in_tx(",
        "release_effect_for_removed_in_tx(",
    ];
    let m = swap_lines(
        SRC,
        SIG_UPDATE,
        ("prior_release_effect_id_in_tx(", 0),
        ("release_effect_for_removed_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_UPDATE, &anchors),
        counts(SRC, SIG_UPDATE, &anchors)
    );
    let d = defects(&m);
    assert!(
        has(&d, "(e) update: prior_release_effect_id_in_tx"),
        "{d:?}"
    );
    assert!(!has(&d, "(cnt)"), "{d:?}");
}

// (8) update_return: prior_release_effect_id_in_tx <-> restore_from_snapshot_in_tx.
#[test]
fn plan_order_guard_mutant_8_update_return_prior_release_after_snapshot() {
    let anchors = [
        "prior_release_effect_id_in_tx(",
        "restore_from_snapshot_in_tx(",
    ];
    let m = swap_lines(
        SRC,
        SIG_UPDATE_RETURN,
        ("prior_release_effect_id_in_tx(", 0),
        ("restore_from_snapshot_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_UPDATE_RETURN, &anchors),
        counts(SRC, SIG_UPDATE_RETURN, &anchors)
    );
    let d = defects(&m);
    assert!(
        has(&d, "(e) update_return: prior_release_effect_id_in_tx"),
        "{d:?}"
    );
    assert!(!has(&d, "(cnt)"), "{d:?}");
}

// (9), (10): повторное чтение эффектов после release_if_locked.
#[test]
fn plan_order_guard_mutant_9_10_effects_reread_after_release() {
    for (sig, label) in [(SIG_UPDATE, "update"), (SIG_UPDATE_RETURN, "update_return")] {
        let a = "list_act_group_effects_in_tx(";
        let m = insert_after(
            SRC,
            sig,
            ("release_if_locked_device_in_tx(", 0),
            "let _reread = group_deps.groups.list_act_group_effects_in_tx(&tx, payload.id)?;",
        );
        assert_eq!(counts(&m, sig, &[a])[0], counts(SRC, sig, &[a])[0] + 1);
        let d = defects(&m);
        assert!(has(&d, &format!("(e2) {label}")), "{d:?}");
    }
}

// (11) do_return: plan_for_return_in_tx <-> insert_act_in_tx (первая запись).
#[test]
fn plan_order_guard_mutant_11_do_return_plan_after_insert() {
    let anchors = ["plan_for_return_in_tx(", "insert_act_in_tx("];
    let m = swap_lines(
        SRC,
        SIG_DO_RETURN,
        ("plan_for_return_in_tx(", 0),
        ("insert_act_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_DO_RETURN, &anchors),
        counts(SRC, SIG_DO_RETURN, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(a) do_return"), "{d:?}");
    assert!(!has(&d, "(cnt)"), "{d:?}");
}

// (12) update: plan_in_tx <-> первый цикл по added (статус-проверка до плана).
#[test]
fn plan_order_guard_mutant_12_update_plan_after_added_loop_and_apply() {
    let anchors = ["plan_in_tx(", "apply_plan_in_tx("];
    // plan <-> apply: план читается после переноса.
    let m = swap_lines(
        SRC,
        SIG_UPDATE,
        ("plan_in_tx(", 0),
        ("apply_plan_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_UPDATE, &anchors),
        counts(SRC, SIG_UPDATE, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(a) update"), "{d:?}");
    assert!(has(&d, "(c) update"), "{d:?}");
}

// (13) update_return: план добавленных читается до ветки removed (нарушает (g)).
#[test]
fn plan_order_guard_mutant_13_update_return_added_plan_before_removed_branch() {
    let anchors = ["plan_for_return_in_tx(", "delete_act_group_effect_in_tx("];
    let m = swap_lines(
        SRC,
        SIG_UPDATE_RETURN,
        ("plan_for_return_in_tx(", 0),
        ("delete_act_group_effect_in_tx(", usize::MAX),
    );
    assert_eq!(
        counts(&m, SIG_UPDATE_RETURN, &anchors),
        counts(SRC, SIG_UPDATE_RETURN, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(g) update_return"), "{d:?}");
}

// (14) update_return: план добавленных ниже его применения.
#[test]
fn plan_order_guard_mutant_14_update_return_plan_after_apply() {
    let anchors = ["plan_for_return_in_tx(", "apply_return_plan_in_tx("];
    let m = swap_lines(
        SRC,
        SIG_UPDATE_RETURN,
        ("plan_for_return_in_tx(", 0),
        ("apply_return_plan_in_tx(", 0),
    );
    assert_eq!(
        counts(&m, SIG_UPDATE_RETURN, &anchors),
        counts(SRC, SIG_UPDATE_RETURN, &anchors)
    );
    let d = defects(&m);
    assert!(has(&d, "(d) update_return стадия 0"), "{d:?}");
}
