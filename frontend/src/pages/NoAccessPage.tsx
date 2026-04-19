import { ServiceNoAccessPage } from "@pushkind/frontend-shell/noAccess";

import { CrmShell } from "../components/CrmShell";
import { CrmShellFatalState } from "../components/CrmShellFatalState";
import {
  fetchHubMenuItems,
  fetchNoAccessData,
  fetchShellData,
} from "../lib/api";
import type { NoAccessData, ShellData, UserMenuItem } from "../lib/models";

export function NoAccessPage() {
  return (
    <ServiceNoAccessPage<NoAccessData, ShellData, UserMenuItem>
      serviceLabel="CRM"
      fetchShellData={fetchShellData}
      fetchHubMenuItems={fetchHubMenuItems}
      fetchNoAccessData={fetchNoAccessData}
      ShellComponent={CrmShell}
      FatalStateComponent={CrmShellFatalState}
      menuLoadWarning="Failed to load auth navigation menu. Falling back to local CRM menu only."
    />
  );
}
