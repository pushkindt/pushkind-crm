export interface UserMenuItem {
  name: string;
  url: string;
}

type UserMenuDropdownProps = {
  currentUserEmail: string;
  items: UserMenuItem[];
  logoutAction: string;
  homeUrl?: string;
  homeLabel?: string;
};

export function UserMenuDropdown({
  currentUserEmail,
  items,
  logoutAction,
  homeUrl,
  homeLabel = "Домой",
}: UserMenuDropdownProps) {
  const hasNavigationItems = Boolean(homeUrl) || items.length > 0;

  return (
    <div className="dropdown-center">
      <button
        className="btn btn-link nav-link align-items-center text-muted dropdown-toggle"
        type="button"
        data-bs-toggle="dropdown"
        aria-expanded="false"
      >
        <i className="bi bi-person-circle fs-4" />
      </button>
      <ul className="dropdown-menu dropdown-menu-end">
        <li>
          <h6 className="dropdown-header">{currentUserEmail}</h6>
        </li>
        {hasNavigationItems ? (
          <li>
            <hr className="dropdown-divider" />
          </li>
        ) : null}
        {homeUrl ? (
          <li>
            <a className="dropdown-item icon-link" href={homeUrl}>
              <i className="bi bi-house mb-2" />
              {homeLabel}
            </a>
          </li>
        ) : null}
        {items.map((item) => (
          <li key={`${item.url}-${item.name}`}>
            <a className="dropdown-item icon-link" href={item.url}>
              <i className="bi bi-grid mb-2" />
              {item.name}
            </a>
          </li>
        ))}
        <li>
          <form method="POST" action={logoutAction}>
            <button type="submit" className="dropdown-item icon-link">
              <i className="bi bi-box-arrow-right mb-2" />
              Выйти
            </button>
          </form>
        </li>
      </ul>
    </div>
  );
}
