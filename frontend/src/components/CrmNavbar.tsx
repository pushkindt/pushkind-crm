import type { ReactNode } from "react";

import { UserMenuDropdown } from "./UserMenuDropdown";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type CrmNavbarProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  menuItems: UserMenuItem[];
  search?: ReactNode;
};

export function CrmNavbar({
  navigation,
  currentUserEmail,
  homeUrl,
  menuItems,
  search,
}: CrmNavbarProps) {
  return (
    <div className="container pt-2">
      <nav className="navbar navbar-expand-sm bg-body-tertiary crm-navbar">
        <div className="container-fluid">
          <a className="navbar-brand" href="/">
            CRM
          </a>
          <button
            className="navbar-toggler"
            type="button"
            data-bs-toggle="collapse"
            data-bs-target="#crm-foundation-navbar"
            aria-controls="crm-foundation-navbar"
            aria-expanded="false"
            aria-label="Toggle navigation"
          >
            <span className="navbar-toggler-icon" />
          </button>
          <div className="collapse navbar-collapse" id="crm-foundation-navbar">
            <ul className="navbar-nav me-auto">
              {navigation.map((item) => (
                <li className="nav-item" key={item.url}>
                  <a className="nav-link" href={item.url}>
                    {item.name}
                  </a>
                </li>
              ))}
            </ul>
            {search ? <div className="crm-navbar-search">{search}</div> : null}
          </div>
          <div className="ms-sm-2">
            <UserMenuDropdown
              currentUserEmail={currentUserEmail}
              items={menuItems}
              homeUrl={homeUrl}
              logoutAction="/logout"
            />
          </div>
        </div>
      </nav>
    </div>
  );
}
