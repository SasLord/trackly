// Phase 3 Plan 04: Organization API wrapper.
//
// Backend: organization_get → OrgDto (snake_case JSON).

import { apiCall } from './client';
import type { OrgDto } from '../../bindings';

export const organization = {
  get: () => apiCall<OrgDto>('organization_get'),
};
