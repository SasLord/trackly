// Phase 3 Plan 04: Templates API wrapper.
//
// Backend: templates_get_active (kind → body string),
//          templates_render_preview (kind + sample_act_id → number[] PDF bytes).
// Used by the future Phase 7 template editor.

import { apiCall } from './client';

export const templates = {
  getActive: (kind: string) => apiCall<string>('templates_get_active', { kind }),
  renderPreview: (kind: string, sampleActId: number) =>
    apiCall<number[]>('templates_render_preview', { kind, sampleActId }),
};
