// Phase 3 Plan 04: PDF helpers — convert backend Vec<u8> → Blob + manage URL lifecycle.
//
// Tauri serializes Vec<u8> as `number[]`. We wrap in Uint8Array → Blob with
// `application/pdf` mime type. The created object URL must be revoked by the
// caller in $effect cleanup (PdfPreviewModal does this).

export async function fetchPdfBlob(
  promise: Promise<number[]>,
): Promise<{ blob: Blob; url: string }> {
  const bytes = await promise;
  const buf = new Uint8Array(bytes);
  const blob = new Blob([buf], { type: 'application/pdf' });
  const url = URL.createObjectURL(blob);
  return { blob, url };
}

export function revokePdfUrl(url: string | null) {
  if (url) URL.revokeObjectURL(url);
}
