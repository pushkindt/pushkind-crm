import { ModalFlashShell } from "@pushkind/frontend-shell/ModalFlashShell";
import type { ReactNode } from "react";

import { CrmNavbar } from "./CrmNavbar";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type CrmShellProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
  children: ReactNode;
};

export function CrmShell({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
  children,
}: CrmShellProps) {
  return (
    <ModalFlashShell
      navbar={
        <CrmNavbar
          navigation={navigation}
          currentUserEmail={currentUserEmail}
          homeUrl={homeUrl}
          localMenuItems={localMenuItems}
          fetchedMenuItems={fetchedMenuItems}
          search={search}
        />
      }
    >
      {children}
    </ModalFlashShell>
  );
}
