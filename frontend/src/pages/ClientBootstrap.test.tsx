import { describe, expect, it } from "vitest";

import { localizeTaskPriority, localizeTaskStatus } from "./ClientBootstrap";

describe("ClientBootstrap task copy", () => {
  it("localizes known task statuses to Russian copy", () => {
    expect(localizeTaskStatus("Pending")).toBe("Ожидает");
    expect(localizeTaskStatus("InProgress")).toBe("В работе");
    expect(localizeTaskStatus("Blocked")).toBe("Заблокирована");
    expect(localizeTaskStatus("Completed")).toBe("Завершена");
    expect(localizeTaskStatus("Archived")).toBe("В архиве");
  });

  it("localizes known task priorities to Russian copy", () => {
    expect(localizeTaskPriority("Low")).toBe("Низкий");
    expect(localizeTaskPriority("Middle")).toBe("Средний");
    expect(localizeTaskPriority("High")).toBe("Высокий");
  });

  it("preserves unknown task values without crashing", () => {
    expect(localizeTaskStatus("CustomStatus")).toBe("CustomStatus");
    expect(localizeTaskPriority("Urgent")).toBe("Urgent");
  });
});
