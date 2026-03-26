import { afterEach, describe, expect, it, vi } from "vitest";

import { browserLocation, postForm } from "./api";

describe("mutation auth redirect handling", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("navigates to the redirected auth page before JSON parsing", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue({
      redirected: true,
      url: "https://users.pushkind.com/auth/signin?next=%2Fclient%2F1",
      status: 200,
      ok: true,
      headers: new Headers({ "content-type": "text/html; charset=utf-8" }),
      json: vi.fn(),
    } as unknown as Response);
    const assignSpy = vi
      .spyOn(browserLocation, "assign")
      .mockImplementation(() => undefined);

    await expect(
      postForm("/client/1/save", new URLSearchParams()),
    ).rejects.toThrow("Сессия истекла.");

    expect(fetchMock).toHaveBeenCalledOnce();
    expect(assignSpy).toHaveBeenCalledWith(
      "https://users.pushkind.com/auth/signin?next=%2Fclient%2F1",
    );
  });
});
