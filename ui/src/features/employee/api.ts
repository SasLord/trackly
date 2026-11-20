// Раздел «Устройства» сотрудника (фаза 41.6, план 08): обёртка единственной
// read-команды. Типы только через `import type` (bindings.ts в рантайме не
// импортируется). Команда есть в allowlist сотрудника (employee-allowed-commands.json).

import { apiCall } from '$lib/api/client';
import type { EmployeeDevicesDto } from '../../bindings';

export const employeeDevices = {
  list: () => apiCall<EmployeeDevicesDto>('employee_devices_list'),
};
