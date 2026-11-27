// Раздел «Link» (фаза 44, план 18): типизированные обёртки над админ-командами Link.
// Идут через apiCall (Tauri invoke или HTTP) — bindings.ts в рантайме не импортируется,
// только типы.
import { apiCall } from '@trackly/transport/client';
import type {
  BannedDeviceDto,
  CreateInviteRequest,
  InviteIssuedDto,
  InviteRowDto,
  LinkLoginRowDto,
  RevokeResultDto,
  UserLinkStatusDto,
} from '../../bindings';

export const linkApi = {
  invitesCreate: (req: CreateInviteRequest) =>
    apiCall<InviteIssuedDto>('link_invites_create', { req }),
  invitesList: () => apiCall<InviteRowDto[]>('link_invites_list', {}),
  invitesRevoke: (id: number) => apiCall<null>('link_invites_revoke', { id }),
  loginsList: () => apiCall<LinkLoginRowDto[]>('link_logins_list', {}),
  loginsRevoke: (loginId: number) => apiCall<null>('link_logins_revoke', { loginId }),
  devicesList: () => apiCall<BannedDeviceDto[]>('link_devices_list', {}),
  devicesAllow: (deviceId: string) => apiCall<null>('link_devices_allow', { deviceId }),
  userStatus: (userId: number) => apiCall<UserLinkStatusDto>('users_link_status', { userId }),
  userRevoke: (userId: number) => apiCall<RevokeResultDto>('users_revoke_link', { userId }),
};
