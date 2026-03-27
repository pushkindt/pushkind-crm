import { useEffect, useState } from "react";
import type { ChangeEvent, FormEvent } from "react";

import { CrmShell } from "../components/CrmShell";
import { CrmShellFatalState } from "../components/CrmShellFatalState";
import {
  fetchClientDirectoryData,
  isApiMutationError,
  postForm,
  postMultipartForm,
  toFieldErrorMap,
} from "../lib/api";
import type { ClientDirectoryData } from "../lib/models";
import { useCrmShell } from "../lib/useCrmShell";

type DashboardState =
  | { status: "loading" }
  | { status: "ready"; data: ClientDirectoryData }
  | { status: "error"; message: string };

export function DashboardBootstrap() {
  const shellState = useCrmShell("Не удалось загрузить React-оболочку CRM.");
  const [dashboardState, setDashboardState] = useState<DashboardState>({
    status: "loading",
  });
  const [addClientErrors, setAddClientErrors] = useState<
    Record<string, string>
  >({});
  const [isAddClientSubmitting, setIsAddClientSubmitting] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [isUploadSubmitting, setIsUploadSubmitting] = useState(false);
  const canAddClient =
    shellState.status === "ready"
      ? shellState.shell.currentUser.roles.includes("crm_admin")
      : false;

  const loadDashboard = async () => {
    const data = await fetchClientDirectoryData(
      new URLSearchParams(window.location.search),
    );
    setDashboardState({ status: "ready", data });
  };

  useEffect(() => {
    let active = true;

    void fetchClientDirectoryData(new URLSearchParams(window.location.search))
      .then((data) => {
        if (!active) {
          return;
        }

        setDashboardState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setDashboardState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить данные React-предпросмотра CRM.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

  if (shellState.status === "loading" || dashboardState.status === "loading") {
    return null;
  }

  if (shellState.status === "error") {
    return <CrmShellFatalState message={shellState.message} />;
  }

  if (dashboardState.status === "error") {
    return <CrmShellFatalState message={dashboardState.message} />;
  }

  async function handleAddClientSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    setIsAddClientSubmitting(true);
    setAddClientErrors({});
    const body = new URLSearchParams();
    for (const [key, value] of new FormData(form).entries()) {
      body.append(key, String(value));
    }

    try {
      const result = await postForm("/client/add", body);
      window.showFlashMessage?.(result.message, "success");
      window.bootstrap?.Modal.getOrCreateInstance("#clientModal", {}).hide();
      form.reset();
      try {
        await loadDashboard();
      } catch (error) {
        console.error(
          "Failed to refresh dashboard after adding client.",
          error,
        );
        window.location.reload();
      }
    } catch (error) {
      if (isApiMutationError(error)) {
        setAddClientErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
        return;
      }

      console.error("Failed to submit add-client form.", error);
      window.showFlashMessage?.("Не удалось добавить клиента.", "danger");
    } finally {
      setIsAddClientSubmitting(false);
    }
  }

  async function handleUploadSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    setIsUploadSubmitting(true);
    setUploadError(null);

    try {
      const result = await postMultipartForm(
        "/clients/upload",
        new FormData(form),
      );
      window.showFlashMessage?.(result.message, "success");
      form.reset();
      try {
        await loadDashboard();
      } catch (error) {
        console.error(
          "Failed to refresh dashboard after uploading clients.",
          error,
        );
        window.location.reload();
      }
    } catch (error) {
      if (isApiMutationError(error)) {
        setUploadError(error.message);
        window.showFlashMessage?.(error.message, "danger");
        return;
      }

      console.error("Failed to upload clients.", error);
      setUploadError("Не удалось загрузить клиентов.");
      window.showFlashMessage?.("Не удалось загрузить клиентов.", "danger");
    } finally {
      setIsUploadSubmitting(false);
    }
  }

  function clearAddClientError(field: string) {
    setAddClientErrors((current) => ({ ...current, [field]: "" }));
  }

  function handleAddClientFieldChange(event: ChangeEvent<HTMLInputElement>) {
    clearAddClientError(event.target.name);
  }

  return (
    <CrmShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      menuItems={[
        ...shellState.shell.localMenuItems,
        ...shellState.authMenuItems,
      ]}
      search={
        <form className="d-flex w-100" role="search" action="/">
          <div className="input-group me-2">
            <input
              required
              name="search"
              className="form-control"
              type="search"
              placeholder="Поиск"
              aria-label="Search"
              defaultValue={dashboardState.data.searchQuery ?? ""}
            />
            <button className="btn btn-outline-secondary" type="submit">
              <i className="bi bi-search" />
            </button>
          </div>
        </form>
      }
    >
      <>
        <div className="container bg-white border rounded my-2">
          {canAddClient ? (
            <div className="row">
              <div className="col text-center add-client-container">
                <button
                  className="btn btn-link"
                  type="button"
                  data-bs-toggle="modal"
                  data-bs-target="#clientModal"
                >
                  <i className="bi bi-plus-circle" />
                </button>
              </div>
            </div>
          ) : null}

          <div className="row d-none d-sm-flex fw-bold">
            <div className="col overflow-hidden">Название</div>
            <div className="col overflow-hidden">Электронный адрес</div>
            <div className="col overflow-hidden">Телефон</div>
            <div className="col overflow-hidden">Дополнительные поля</div>
          </div>
          <div id="clientList">
            {dashboardState.data.clients.items.length > 0 ? (
              dashboardState.data.clients.items.map((client) => (
                <a
                  className="row my-1 py-1 border-top client selectable text-decoration-none text-body"
                  data-id={client.id}
                  href={`/client/${client.id}`}
                  key={client.id}
                >
                  <div className="col-sm overflow-hidden">
                    <strong>{client.name}</strong>
                  </div>
                  <div className="col-sm overflow-hidden">
                    {client.email ?? "—"}
                  </div>
                  <div className="col-sm overflow-hidden">
                    {client.phone ?? "—"}
                  </div>
                  <div className="col-sm overflow-hidden">
                    {client.fieldBadges.length > 0 ? (
                      client.fieldBadges.map((badge) => (
                        <span
                          className="badge rounded-pill text-bg-light me-1"
                          key={`${client.id}-${badge}`}
                        >
                          {badge}
                        </span>
                      ))
                    ) : (
                      <span className="text-secondary">—</span>
                    )}
                  </div>
                </a>
              ))
            ) : (
              <div className="alert alert-warning my-2" role="alert">
                Нет клиентов для отображения.
              </div>
            )}
          </div>
          {dashboardState.data.clients.pages.length > 1 ? (
            <nav aria-label="pagination" className="py-3">
              <ul className="pagination justify-content-center flex-wrap mb-0">
                {dashboardState.data.clients.pages.map((page, index) =>
                  page ? (
                    page !== dashboardState.data.clients.page ? (
                      <li className="page-item" key={`${page}-${index}`}>
                        <a
                          className="page-link"
                          href={`/?page=${page}${
                            dashboardState.data.searchQuery
                              ? `&search=${encodeURIComponent(
                                  dashboardState.data.searchQuery,
                                )}`
                              : ""
                          }`}
                        >
                          {page}
                        </a>
                      </li>
                    ) : (
                      <li
                        className="page-item active"
                        aria-current="page"
                        key={`${page}-${index}`}
                      >
                        <span className="page-link">{page}</span>
                      </li>
                    )
                  ) : (
                    <li className="page-item" key={`gap-${index}`}>
                      <span className="ellipsis px-2">…</span>
                    </li>
                  ),
                )}
              </ul>
            </nav>
          ) : null}
        </div>

        {canAddClient ? (
          <div
            className="modal fade"
            id="clientModal"
            tabIndex={-1}
            aria-labelledby="clientModalLabel"
            aria-hidden="true"
          >
            <div className="modal-dialog modal-lg">
              <div className="modal-content">
                <div className="modal-header">
                  <h1 className="modal-title fs-5" id="clientModalLabel">
                    Добавить клиента
                  </h1>
                  <button
                    type="button"
                    className="btn-close"
                    data-bs-dismiss="modal"
                    aria-label="Close"
                  />
                </div>
                <div className="modal-body">
                  <form onSubmit={(event) => void handleAddClientSubmit(event)}>
                    <div className="row mb-3">
                      <label
                        htmlFor="clientModalName"
                        className="col-md-2 col-form-label"
                      >
                        Имя
                      </label>
                      <div className="col-md-10">
                        <input
                          name="name"
                          type="text"
                          className={
                            addClientErrors.name
                              ? "form-control is-invalid"
                              : "form-control"
                          }
                          id="clientModalName"
                          placeholder="Имя"
                          required
                          onChange={handleAddClientFieldChange}
                        />
                        {addClientErrors.name ? (
                          <div className="invalid-feedback">
                            {addClientErrors.name}
                          </div>
                        ) : null}
                      </div>
                    </div>
                    <div className="row mb-3">
                      <label
                        htmlFor="clientModalEmail"
                        className="col-md-2 col-form-label"
                      >
                        Электронный адрес
                      </label>
                      <div className="col-md-10">
                        <input
                          name="email"
                          type="email"
                          className={
                            addClientErrors.email
                              ? "form-control is-invalid"
                              : "form-control"
                          }
                          id="clientModalEmail"
                          placeholder="Электронный адрес"
                          onChange={handleAddClientFieldChange}
                        />
                        {addClientErrors.email ? (
                          <div className="invalid-feedback">
                            {addClientErrors.email}
                          </div>
                        ) : null}
                      </div>
                    </div>
                    <div className="row mb-3">
                      <label
                        htmlFor="clientModalPhone"
                        className="col-md-2 col-form-label"
                      >
                        Телефон
                      </label>
                      <div className="col-md-10">
                        <input
                          name="phone"
                          type="tel"
                          className={
                            addClientErrors.phone
                              ? "form-control is-invalid"
                              : "form-control"
                          }
                          id="clientModalPhone"
                          placeholder="Телефон"
                          onChange={handleAddClientFieldChange}
                        />
                        {addClientErrors.phone ? (
                          <div className="invalid-feedback">
                            {addClientErrors.phone}
                          </div>
                        ) : null}
                      </div>
                    </div>
                    <div className="row mb-3">
                      <div className="col">
                        <button
                          className="btn btn-primary"
                          type="submit"
                          disabled={isAddClientSubmitting}
                        >
                          Сохранить
                        </button>
                      </div>
                    </div>
                  </form>
                </div>
                <div className="modal-footer">
                  <form
                    className="w-100"
                    onSubmit={(event) => void handleUploadSubmit(event)}
                  >
                    <div className="row">
                      <div className="col">
                        <input
                          className="form-control"
                          type="file"
                          name="csv"
                          accept=".csv"
                          required
                          onChange={() => setUploadError(null)}
                        />
                        {uploadError ? (
                          <div className="text-danger small mt-1">
                            {uploadError}
                          </div>
                        ) : null}
                        <div className="w-100">
                          <sup>
                            <small className="text-muted">
                              "name","email","phone","произвольные","поля"
                            </small>
                          </sup>
                        </div>
                      </div>
                      <div className="col-auto">
                        <button
                          className="btn btn-success"
                          type="submit"
                          disabled={isUploadSubmitting}
                        >
                          Из csv
                        </button>
                      </div>
                    </div>
                  </form>
                </div>
              </div>
            </div>
          </div>
        ) : null}
      </>
    </CrmShell>
  );
}
