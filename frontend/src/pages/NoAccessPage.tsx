import { useEffect, useState } from "react";

type NoAccessData = {
  currentUser: {
    email: string;
    name: string;
  };
  homeUrl: string;
};

function parseNoAccessData(payload: unknown): NoAccessData {
  if (
    typeof payload !== "object" ||
    payload === null ||
    typeof (payload as { home_url?: unknown }).home_url !== "string" ||
    typeof (payload as { current_user?: unknown }).current_user !== "object" ||
    (payload as { current_user: { email?: unknown } }).current_user === null ||
    typeof (payload as { current_user: { email?: unknown } }).current_user
      .email !== "string" ||
    typeof (payload as { current_user: { name?: unknown } }).current_user
      .name !== "string"
  ) {
    throw new Error("Invalid no-access payload.");
  }

  const typedPayload = payload as {
    home_url: string;
    current_user: {
      email: string;
      name: string;
    };
  };

  return {
    homeUrl: typedPayload.home_url,
    currentUser: {
      email: typedPayload.current_user.email,
      name: typedPayload.current_user.name,
    },
  };
}

export function NoAccessPage() {
  const [state, setState] = useState<
    | { status: "loading" }
    | { status: "ready"; data: NoAccessData }
    | { status: "error"; message: string }
  >({ status: "loading" });

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        const response = await fetch("/api/v1/no-access", {
          credentials: "include",
        });
        if (!response.ok) {
          throw new Error(`Request failed with status ${response.status}.`);
        }

        const data = parseNoAccessData(await response.json());
        if (!active) {
          return;
        }

        setState({ status: "ready", data });
      } catch (error) {
        if (!active) {
          return;
        }

        setState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу.",
        });
      }
    }

    void load();

    return () => {
      active = false;
    };
  }, []);

  if (state.status === "loading") {
    return (
      <main className="container py-5">
        <div className="alert alert-secondary mb-0" role="status">
          Загрузка...
        </div>
      </main>
    );
  }

  if (state.status === "error") {
    return (
      <main className="container py-5">
        <div className="alert alert-danger mb-0" role="alert">
          {state.message}
        </div>
      </main>
    );
  }

  return (
    <main className="container py-5">
      <div className="card shadow-sm">
        <div className="card-body p-4">
          <p className="text-uppercase text-secondary small mb-2">CRM</p>
          <h1 className="h3 mb-3">Недостаточно прав для доступа к сервису</h1>
          <p className="text-secondary mb-4">
            Пользователь <strong>{state.data.currentUser.name}</strong> не имеет
            роли <code>crm</code>.
          </p>
          <div className="d-flex flex-column flex-sm-row gap-2">
            <a className="btn btn-primary" href={state.data.homeUrl}>
              На главную
            </a>
            <form method="POST" action="/logout">
              <button className="btn btn-outline-secondary" type="submit">
                Выйти
              </button>
            </form>
          </div>
        </div>
      </div>
    </main>
  );
}
