import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { UserMenuDropdown } from "./UserMenuDropdown";

describe("UserMenuDropdown", () => {
  it("renders local items before fetched items and keeps logout last", () => {
    const markup = renderToStaticMarkup(
      <UserMenuDropdown
        currentUserEmail="user@example.com"
        localItems={[
          { name: "Домой", url: "https://users.pushkind.com" },
          { name: "Настройки", url: "/settings" },
        ]}
        fetchedItems={[
          { name: "CRM", url: "/crm" },
          { name: "Отчеты", url: "https://reports.pushkind.com" },
          { name: "Выйти", url: "https://auth.pushkind.com/logout" },
        ]}
        logoutAction="/logout"
      />,
    );

    expect(markup).toContain("user@example.com");
    expect(markup).toContain("https://users.pushkind.com");
    expect(markup).toContain("Домой");
    expect(markup).toContain("bi bi-house mb-2");
    expect(markup.indexOf("Домой")).toBeLessThan(markup.indexOf("Настройки"));
    expect(markup.indexOf("Настройки")).toBeLessThan(markup.indexOf("CRM"));
    expect(markup.indexOf("CRM")).toBeLessThan(markup.indexOf("Отчеты"));
    expect(markup.indexOf("Отчеты")).toBeLessThan(markup.lastIndexOf("Выйти"));
    expect(markup).toContain("/crm");
    expect(markup).toContain("CRM");
    expect(markup).toContain("/settings");
    expect(markup).toContain("Настройки");
    expect(markup).toContain("bi bi-gear mb-2");
    expect(markup).toContain("https://reports.pushkind.com");
    expect(markup).toContain("/logout");
    expect(markup).toContain("Выйти");
    expect(markup.match(/Выйти/g)?.length).toBe(1);
  });
});
