import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { UserMenuDropdown } from "./UserMenuDropdown";

describe("UserMenuDropdown", () => {
  it("renders the home link, menu items, and logout action", () => {
    const markup = renderToStaticMarkup(
      <UserMenuDropdown
        currentUserEmail="user@example.com"
        homeUrl="https://users.pushkind.com"
        items={[{ name: "CRM", url: "/crm" }]}
        logoutAction="/logout"
      />,
    );

    expect(markup).toContain("user@example.com");
    expect(markup).toContain("https://users.pushkind.com");
    expect(markup).toContain("Домой");
    expect(markup).toContain("/crm");
    expect(markup).toContain("CRM");
    expect(markup).toContain("/logout");
    expect(markup).toContain("Выйти");
  });
});
