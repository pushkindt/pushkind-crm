import type { ReactNode } from "react";
import { ServiceNavbar } from "@pushkind/frontend-shell/ServiceNavbar";

import type { NavigationItem, UserMenuItem } from "../lib/models";

type CrmNavbarProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
};

export function CrmNavbar({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
}: CrmNavbarProps) {
  return (
    <ServiceNavbar
      brand="CRM"
      collapseId="crm-foundation-navbar"
      navigation={navigation}
      currentUserEmail={currentUserEmail}
      homeUrl={homeUrl}
      localMenuItems={localMenuItems}
      fetchedMenuItems={fetchedMenuItems}
      logoutAction="/logout"
      outerContainerClassName="container pt-2"
      navbarClassName="crm-navbar"
      search={search}
      searchWrapperClassName="crm-navbar-search"
    />
  );
}
