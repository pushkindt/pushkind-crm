import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";

import { CrmShell } from "../components/CrmShell";
import { CrmShellFatalState } from "../components/CrmShellFatalState";
import {
  fetchAuthUsers,
  fetchClientsData,
  isApiMutationError,
  fetchManagerModalData,
  fetchManagersData,
  postForm,
  toFieldErrorMap,
} from "../lib/api";
import type {
  AuthUserSearchItem,
  ClientListItem,
  ManagerModalData,
  ManagersData,
} from "../lib/models";
import { useCrmShell } from "../lib/useCrmShell";

type ManagersState =
  | { status: "loading" }
  | { status: "ready"; data: ManagersData }
  | { status: "error"; message: string };

type SearchState =
  | { status: "idle"; items: AuthUserSearchItem[] }
  | { status: "loading"; items: AuthUserSearchItem[] }
  | { status: "error"; items: AuthUserSearchItem[] };

type ClientSearchState =
  | { status: "idle"; items: ClientListItem[] }
  | { status: "loading"; items: ClientListItem[] }
  | { status: "error"; items: ClientListItem[] };

type ManagerModalState =
  | { status: "idle" }
  | { status: "loading"; managerName: string }
  | { status: "ready"; data: ManagerModalData }
  | { status: "error"; managerName: string; message: string };

export function ManagersBootstrap() {
  const shellState = useCrmShell("Не удалось загрузить React-оболочку CRM.");
  const [managersState, setManagersState] = useState<ManagersState>({
    status: "loading",
  });
  const [managerQuery, setManagerQuery] = useState("");
  const [managerSearchState, setManagerSearchState] = useState<SearchState>({
    status: "idle",
    items: [],
  });
  const [selectedUser, setSelectedUser] = useState<AuthUserSearchItem | null>(
    null,
  );
  const [managerModalState, setManagerModalState] = useState<ManagerModalState>(
    { status: "idle" },
  );
  const [clientQuery, setClientQuery] = useState("");
  const [clientSearchState, setClientSearchState] = useState<ClientSearchState>(
    {
      status: "idle",
      items: [],
    },
  );
  const [selectedClientIds, setSelectedClientIds] = useState<number[]>([]);
  const [addManagerErrors, setAddManagerErrors] = useState<
    Record<string, string>
  >({});
  const [assignErrors, setAssignErrors] = useState<Record<string, string>>({});
  const [isAddManagerSubmitting, setIsAddManagerSubmitting] = useState(false);
  const [isAssignSubmitting, setIsAssignSubmitting] = useState(false);
  const managerModalRequestId = useRef(0);

  const loadManagers = async () => {
    const data = await fetchManagersData();
    setManagersState({ status: "ready", data });
  };

  useEffect(() => {
    let active = true;

    void fetchManagersData()
      .then((data) => {
        if (!active) {
          return;
        }

        setManagersState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setManagersState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу менеджеров.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (shellState.status !== "ready") {
      return;
    }

    const query = managerQuery.trim();
    if (query.length === 0) {
      setManagerSearchState({ status: "idle", items: [] });
      return;
    }

    let active = true;
    setManagerSearchState((current) => ({
      status: "loading",
      items: current.items,
    }));

    const timeout = window.setTimeout(() => {
      void fetchAuthUsers(shellState.shell.homeUrl, query)
        .then((items) => {
          if (!active) {
            return;
          }

          setManagerSearchState({ status: "idle", items });
        })
        .catch(() => {
          if (!active) {
            return;
          }

          setManagerSearchState({ status: "error", items: [] });
        });
    }, 200);

    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [managerQuery, shellState]);

  useEffect(() => {
    const query = clientQuery.trim();
    if (managerModalState.status !== "ready" || query.length === 0) {
      setClientSearchState({ status: "idle", items: [] });
      return;
    }

    let active = true;
    setClientSearchState((current) => ({
      status: "loading",
      items: current.items,
    }));

    const timeout = window.setTimeout(() => {
      void fetchClientsData(query)
        .then((items) => {
          if (!active) {
            return;
          }

          setClientSearchState({ status: "idle", items });
        })
        .catch(() => {
          if (!active) {
            return;
          }

          setClientSearchState({ status: "error", items: [] });
        });
    }, 200);

    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [clientQuery, managerModalState]);

  const selectedClients = useMemo(() => {
    if (managerModalState.status !== "ready") {
      return [];
    }

    return managerModalState.data.clients.filter((client) =>
      selectedClientIds.includes(client.id),
    );
  }, [managerModalState, selectedClientIds]);

  if (shellState.status === "loading" || managersState.status === "loading") {
    return null;
  }

  if (shellState.status === "error") {
    return <CrmShellFatalState message={shellState.message} />;
  }

  if (managersState.status === "error") {
    return <CrmShellFatalState message={managersState.message} />;
  }

  async function handleAddManagerSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsAddManagerSubmitting(true);
    setAddManagerErrors({});

    const body = new URLSearchParams();
    body.set("name", selectedUser?.name ?? "");
    body.set("email", selectedUser?.email ?? "");

    try {
      const result = await postForm("/managers/add", body);
      window.showFlashMessage?.(result.message, "success");
      setSelectedUser(null);
      setManagerQuery("");
      setManagerSearchState({ status: "idle", items: [] });
      await loadManagers();
    } catch (error) {
      if (isApiMutationError(error)) {
        setAddManagerErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to add manager.", error);
        window.showFlashMessage?.("Не удалось добавить менеджера.", "danger");
      }
    } finally {
      setIsAddManagerSubmitting(false);
    }
  }

  async function handleAssignSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (managerModalState.status !== "ready") {
      return;
    }

    setIsAssignSubmitting(true);
    setAssignErrors({});

    const body = new URLSearchParams();
    body.set("manager_id", String(managerModalState.data.manager.id));
    selectedClientIds.forEach((id) => body.append("client_ids", String(id)));

    try {
      const result = await postForm("/managers/assign", body);
      window.showFlashMessage?.(result.message, "success");
      window.bootstrap?.Modal.getOrCreateInstance("#managerModal", {}).hide();
      setClientQuery("");
      await loadManagers();
    } catch (error) {
      if (isApiMutationError(error)) {
        setAssignErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to assign manager clients.", error);
        window.showFlashMessage?.(
          "Не удалось сохранить назначения менеджера.",
          "danger",
        );
      }
    } finally {
      setIsAssignSubmitting(false);
    }
  }

  function handleManagerModalOpen(managerId: number, managerName: string) {
    managerModalRequestId.current += 1;
    const requestId = managerModalRequestId.current;

    setAssignErrors({});
    setClientQuery("");
    setClientSearchState({ status: "idle", items: [] });
    setSelectedClientIds([]);
    setManagerModalState({ status: "loading", managerName });

    void fetchManagerModalData(managerId)
      .then((data) => {
        if (managerModalRequestId.current !== requestId) {
          return;
        }

        setManagerModalState({ status: "ready", data });
        setSelectedClientIds(data.clients.map((client) => client.id));
      })
      .catch((error) => {
        if (managerModalRequestId.current !== requestId) {
          return;
        }

        console.error(`Failed to load manager modal for ${managerId}.`, error);
        setManagerModalState({
          status: "error",
          managerName,
          message: "Не удалось загрузить назначения менеджера.",
        });
      });
  }

  return (
    <CrmShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
    >
      <div className="container my-2">
        <div className="row">
          <div className="col">
            <form onSubmit={(event) => void handleAddManagerSubmit(event)}>
              <div className="row">
                <div className="col">
                  <input
                    className="form-control my-1"
                    placeholder="Добавить менеджера"
                    value={selectedUser ? selectedUser.name : managerQuery}
                    onChange={(event) => {
                      setSelectedUser(null);
                      setManagerQuery(event.target.value);
                    }}
                    required
                  />
                  {addManagerErrors.name || addManagerErrors.email ? (
                    <div className="text-danger small mt-1">
                      {addManagerErrors.name || addManagerErrors.email}
                    </div>
                  ) : null}
                  <input
                    type="hidden"
                    name="name"
                    value={selectedUser?.name ?? ""}
                    required
                    readOnly
                  />
                  <input
                    type="hidden"
                    name="email"
                    value={selectedUser?.email ?? ""}
                    required
                    readOnly
                  />
                  {selectedUser == null &&
                  managerSearchState.items.length > 0 ? (
                    <div className="list-group position-relative shadow-sm">
                      {managerSearchState.items.map((item) => (
                        <button
                          type="button"
                          className="list-group-item list-group-item-action"
                          key={item.sub}
                          onClick={() => {
                            setSelectedUser(item);
                            setManagerQuery(item.name);
                            setManagerSearchState({
                              status: "idle",
                              items: [],
                            });
                          }}
                        >
                          <strong>
                            {item.name} ({item.email})
                          </strong>
                        </button>
                      ))}
                    </div>
                  ) : null}
                </div>
                <div className="col-auto">
                  <button
                    className="btn btn-primary my-1"
                    type="submit"
                    disabled={!selectedUser || isAddManagerSubmitting}
                  >
                    <i className="bi bi-plus" />
                  </button>
                </div>
              </div>
            </form>
          </div>
        </div>
      </div>

      <div className="container border bg-white rounded my-2" id="items">
        <div className="row mb-3 fw-bold pt-3">
          <div className="col overflow-hidden">Имя</div>
          <div className="col overflow-hidden">Email</div>
          <div className="col overflow-hidden">Клиенты</div>
        </div>

        {managersState.data.managers.map(({ manager, clients }) => (
          <button
            type="button"
            className="row mb-3 border-bottom selectable text-start w-100 bg-transparent border-0"
            key={manager.id}
            data-bs-toggle="modal"
            data-bs-target="#managerModal"
            onClick={() => handleManagerModalOpen(manager.id, manager.name)}
          >
            <div className="col overflow-hidden">{manager.name}</div>
            <div className="col overflow-hidden">{manager.email}</div>
            <div className="col overflow-hidden">
              {clients.map((client) => (
                <span
                  className="badge rounded-pill text-bg-light me-1"
                  key={`${manager.id}-${client.id}`}
                >
                  {client.name}
                </span>
              ))}
            </div>
          </button>
        ))}
      </div>

      <div
        className="modal fade"
        id="managerModal"
        tabIndex={-1}
        aria-labelledby="managerModalLabel"
        aria-hidden="true"
      >
        <div className="modal-dialog modal-lg">
          <div className="modal-content">
            <div className="modal-header">
              <h1 className="modal-title fs-5" id="managerModalLabel">
                Назначение клиентов
              </h1>
              <button
                type="button"
                className="btn-close"
                data-bs-dismiss="modal"
                aria-label="Close"
              />
            </div>
            <div className="modal-body">
              {managerModalState.status === "ready" ? (
                <form onSubmit={(event) => void handleAssignSubmit(event)}>
                  <input
                    type="hidden"
                    value={managerModalState.data.manager.id}
                    name="manager_id"
                    required
                    readOnly
                  />
                  {selectedClientIds.map((id) => (
                    <input
                      type="hidden"
                      name="client_ids"
                      value={id}
                      key={`selected-${id}`}
                      readOnly
                    />
                  ))}
                  <div className="row mb-3">
                    <label
                      htmlFor="modalManagerName"
                      className="col-md-2 col-form-label"
                    >
                      Имя
                    </label>
                    <div className="col-md-10">
                      <input
                        type="text"
                        className="form-control-plaintext"
                        id="modalManagerName"
                        value={managerModalState.data.manager.name}
                        readOnly
                      />
                    </div>
                  </div>
                  <div className="row mb-3">
                    <label
                      htmlFor="modalManagerEmail"
                      className="col-md-2 col-form-label"
                    >
                      Электронный адрес
                    </label>
                    <div className="col-md-10">
                      <input
                        type="email"
                        className="form-control-plaintext"
                        id="modalManagerEmail"
                        value={managerModalState.data.manager.email}
                        readOnly
                      />
                    </div>
                  </div>
                  <div className="row mb-3">
                    <label className="col-md-2 col-form-label">Клиенты</label>
                    <div className="col-md-10">
                      <input
                        className="form-control my-1"
                        placeholder="Поиск клиентов"
                        value={clientQuery}
                        onChange={(event) => setClientQuery(event.target.value)}
                      />
                      {clientSearchState.items.length > 0 ? (
                        <div className="list-group mb-3 shadow-sm">
                          {clientSearchState.items.map((client) => (
                            <button
                              type="button"
                              className="list-group-item list-group-item-action"
                              key={client.id}
                              onClick={() => {
                                setSelectedClientIds((current) =>
                                  current.includes(client.id)
                                    ? current
                                    : [...current, client.id],
                                );
                                setManagerModalState((current) =>
                                  current.status === "ready"
                                    ? {
                                        status: "ready",
                                        data: {
                                          ...current.data,
                                          clients: current.data.clients.some(
                                            (item) => item.id === client.id,
                                          )
                                            ? current.data.clients
                                            : [...current.data.clients, client],
                                        },
                                      }
                                    : current,
                                );
                              }}
                            >
                              <strong>{client.name}</strong>
                              <br />
                              <small>
                                {client.email ?? "—"} {client.phone ?? ""}
                              </small>
                            </button>
                          ))}
                        </div>
                      ) : null}

                      <div className="d-flex flex-wrap gap-2">
                        {selectedClients.map((client) => (
                          <span
                            className="badge text-bg-light d-inline-flex align-items-center gap-2"
                            key={client.id}
                          >
                            {client.name}
                            <button
                              type="button"
                              className="btn btn-sm p-0 border-0"
                              onClick={() =>
                                setSelectedClientIds((current) =>
                                  current.filter((id) => id !== client.id),
                                )
                              }
                            >
                              <i className="bi bi-x-circle" />
                            </button>
                          </span>
                        ))}
                      </div>
                      {assignErrors.client_ids || assignErrors.manager_id ? (
                        <div className="text-danger small mt-2">
                          {assignErrors.client_ids || assignErrors.manager_id}
                        </div>
                      ) : null}
                    </div>
                  </div>
                  <div className="row mb-3">
                    <div className="col">
                      <button
                        className="btn btn-primary"
                        type="submit"
                        disabled={isAssignSubmitting}
                      >
                        Сохранить
                      </button>
                    </div>
                  </div>
                </form>
              ) : managerModalState.status === "loading" ? (
                <div className="text-secondary">
                  {`Загрузка назначений для ${managerModalState.managerName}...`}
                </div>
              ) : managerModalState.status === "error" ? (
                <div className="alert alert-danger mb-0" role="alert">
                  {managerModalState.message}
                </div>
              ) : (
                <div className="text-secondary">Выберите менеджера.</div>
              )}
            </div>
          </div>
        </div>
      </div>
    </CrmShell>
  );
}
