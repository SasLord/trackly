// Фабрика обработчика pagedjs «повтор шапки таблицы приложения на каждой
// странице». Вынесена из PdfPreviewModal.svelte: Svelte предупреждает
// (perf_avoid_nested_class) об объявлении класса не на верхнем уровне
// компонента, а базовый `Handler` приходит только из динамического
// `import('pagedjs')`, поэтому класс строится фабрикой.
//
// D-15/D-15a (Phase 36): зеркало RepeatTableHeadHandler из
// ui/src/lib/pdfPreview/bootstrapScript.js — держать в синхроне. Область
// действия строго table.appendix-table (T-36-03) — другие таблицы не трогаем.

type HandlerCtor = new (chunker: unknown, polisher: unknown, caller: unknown) => object;

export function makeRepeatTableHeadHandler(Handler: HandlerCtor) {
  return class RepeatTableHeadHandler extends Handler {
    savedThead: Node | null = null;

    constructor(chunker: unknown, polisher: unknown, caller: unknown) {
      super(chunker, polisher, caller);
      const sourceTable = document.querySelector('table.appendix-table');
      const sourceThead = sourceTable?.querySelector('thead') ?? null;
      this.savedThead = sourceThead ? sourceThead.cloneNode(true) : null;
    }

    // Шапку снимаем с разобранного содержимого (afterParsed; в beforeParsed
    // приходит ещё HTML-строка), а не только с `document`: в
    // LAN-печати (PdfPreviewModal → printViaTopLevel) HTML акта ещё не в DOM,
    // когда Previewer конструирует обработчики, — снимок из конструктора
    // пуст, и шапка не повторялась (стенд LAN v1.4, D7).
    afterParsed(parsed: Pick<HTMLElement, 'querySelector'>) {
      if (typeof parsed?.querySelector !== 'function') return;
      const thead = parsed.querySelector('table.appendix-table thead');
      if (thead) this.savedThead = thead.cloneNode(true);
    }

    afterPageLayout(pageElement: HTMLElement) {
      if (!this.savedThead) return;
      pageElement.querySelectorAll('table.appendix-table').forEach((table) => {
        if (table.querySelector('thead')) return; // already has one — first fragment
        table.insertBefore(this.savedThead!.cloneNode(true), table.firstChild);
      });
    }
  };
}
