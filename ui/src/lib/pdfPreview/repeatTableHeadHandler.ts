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

    afterPageLayout(pageElement: HTMLElement) {
      if (!this.savedThead) return;
      pageElement.querySelectorAll('table.appendix-table').forEach((table) => {
        if (table.querySelector('thead')) return; // already has one — first fragment
        table.insertBefore(this.savedThead!.cloneNode(true), table.firstChild);
      });
    }
  };
}
