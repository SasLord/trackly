# packages/ — общие пакеты фронтенда

Рабочие пакеты pnpm-монорепо: `@trackly/design`, `@trackly/kit`, `@trackly/transport`,
`@trackly/attachments`. Их потребители — `ui/` (Trackly) и `link-ui/` (Trackly Link).
Решение о составе — ADR 0001.

## Правило содержимого

В пакет идёт только код без зависимости от домена Trackly: никаких импортов `ui/`,
`bindings*`, `auth`, `features/*`, `$lib/*`. Гейт `ui/scripts/check-workspace-boundaries.mjs`
(в `pnpm --dir ui lint`) это проверяет. Код, тянущий домен, остаётся в `ui/`.

## Механизм разрешения импортов (спайк A1, решение)

Принят вариант (a): **source-package с wildcard-`exports`**, без алиасов в `vite.config.ts`
и без `paths` в `tsconfig.json`. Пакет не собирается заранее: Vite компилирует `.svelte` и
`.ts` прямо из `packages/<имя>/src` (workspace-символическая ссылка резолвится вне
`node_modules`, поэтому `@sveltejs/vite-plugin-svelte` обрабатывает файлы как исходники).
Спайк на `Button.svelte` (вместе с его зависимостью `Spinner.svelte`): `vite build`
и `svelte-check` зелёные.

Пример `packages/kit/package.json`:

```json
"exports": {
  "./components/*": { "svelte": "./src/components/*", "default": "./src/components/*" },
  "./utils/*": "./src/utils/*.ts",
  "./stores/*": "./src/stores/*.ts"
}
```

Ограничение: `ui/vite.config.ts` и будущий `link-ui/vite.config.ts` обязаны разрешать
`@trackly/*` одинаково — через workspace-зависимость (`"@trackly/kit": "workspace:*"` в
`package.json` потребителя) и без собственных алиасов на эти имена.

## Формат спецификаторов

`@trackly/<пакет>/<подпуть от packages/<пакет>/src>`: для `.svelte` — с расширением
(`@trackly/kit/components/Button.svelte`), для `.ts` — без (`@trackly/kit/utils/date`,
`.svelte.ts` -> `.svelte`: `@trackly/kit/stores/toast.svelte`). Внутри одного пакета
остаются относительные импорты. Реэкспорт-прокладок в `ui/` нет.

Новый подкаталог пакета требует записи в `exports` пакета.

## Переезд файлов

Только кодмодом `ui/scripts/_move-codemod.mjs`:

```
node ui/scripts/_move-codemod.mjs --classify <from>...      # можно ли переносить
node ui/scripts/_move-codemod.mjs --apply <from>=<to>...    # git mv + правка спецификаторов + move-map
```

Пути — относительно корня репозитория. После `--apply`: `pnpm --dir ui lint`,
`pnpm --dir ui svelte-check`, `pnpm --dir ui exec vite build`. Если переехавший файл
был входом гейта вне `walk()` (раздел `pinned` в `gate-coverage.baseline.json`),
в том же коммите перенацелить гейт и осознанно обновить `pinned`.

## Покрытие гейтов кода (спайк A4)

`pnpm --dir ui lint` и `pnpm --dir ui svelte-check` покрывают `packages/*/src` и `link-ui/src`:

- **svelte-check**: `ui/tsconfig.json` `include` расширен на `../packages/*/src/**/*` и
  `../link-ui/src/**/*`. Каждому каталогу нужен `svelte.config.js` (реэкспорт
  `ui/svelte.config.js`): без него svelte-check берёт fallback `svelte-preprocess` и падает
  на `<script lang="ts">`. Файлы `packages/svelte.config.js`, `link-ui/svelte.config.js`.
- **eslint**: ESLint 9 не линтует файлы вне каталога конфига, поэтому `eslint .` в `ui/`
  пакеты не видит. Обёртка `ui/scripts/_eslint-workspaces.mjs` запускает eslint из
  `packages/` (и `link-ui/`, когда появится `src/`) с конфигом-реэкспортом
  `eslint.config.mjs`. Пустой `packages/` — ошибка.
- **prettier**: `prettier --check . ../packages ../link-ui`; prettier ищет конфиг вверх от
  файла, поэтому в `packages/prettier.config.mjs` (и `link-ui/prettier.config.mjs`) лежит
  конфиг, повторяющий `ui/.prettierrc.json` с плагином, резолвленным из `ui/node_modules`.

Покрытие доказано мутациями на `packages/kit/src/components/Button.svelte`:
TS-ошибка -> svelte-check красный; сдвиг отступа -> prettier красный; `var` без
использования -> eslint красный; после возврата всё зелёное.
