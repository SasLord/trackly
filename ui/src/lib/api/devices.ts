import { apiCall } from './client';
import type {
  DeviceDto,
  DeviceNew,
  DevicePatch,
  DeviceFilter,
  Pagination,
  DeviceListResponse,
} from '../../bindings';

export const devices = {
  list: (filter: DeviceFilter, pagination: Pagination) =>
    apiCall<DeviceListResponse>('devices_list', { filter, pagination }),

  get: (id: number) => apiCall<DeviceDto>('devices_get', { id }),

  create: (newDevice: DeviceNew) => apiCall<DeviceDto>('devices_create', { device: newDevice }),

  update: (id: number, version: number, patch: DevicePatch) =>
    apiCall<DeviceDto>('devices_update', { id, version, patch }),

  delete: (id: number, version: number) => apiCall<null>('devices_delete', { id, version }),

  stateHints: () => apiCall<string[]>('devices_state_hints'),
};
