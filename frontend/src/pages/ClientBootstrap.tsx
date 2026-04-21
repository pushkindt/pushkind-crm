import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";
import {
  MarkdownComposer,
  renderMarkdownToHtml,
} from "@pushkind/frontend-shell/markdown";

import { CrmShell } from "../components/CrmShell";
import { CrmShellFatalState } from "../components/CrmShellFatalState";
import {
  fetchClientDetails,
  fetchHubMenuItems,
  fetchShellData,
  isApiMutationError,
  postForm,
  toFieldErrorMap,
} from "../lib/api";
import type {
  ClientDetails,
  ClientEvent,
  Manager,
  ShellData,
  UserMenuItem,
} from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

declare global {
  interface Window {
    mountFileBrowser?: (
      target: string | Element,
      initialPath?: string,
      options?: { baseUrl?: string; historyMode?: "managed" | "disabled" },
    ) => unknown;
  }
}

type ClientState =
  | { status: "loading" }
  | { status: "ready"; data: ClientDetails }
  | { status: "error"; message: string };

type EditableFieldRow = {
  id: string;
  label: string;
  value: string;
  readOnly: boolean;
};

function parseClientIdFromLocation() {
  const match = window.location.pathname.match(/\/client\/(\d+)$/);
  if (!match) {
    throw new Error("Не удалось определить клиента из URL.");
  }

  return Number(match[1]);
}

function renderManagerPopover(manager: Manager, todoServiceUrl: string) {
  const todoLink =
    todoServiceUrl && manager.isUser
      ? `<br><a class='text-decoration-none' href='${todoServiceUrl}?email=${encodeURIComponent(
          manager.email,
        )}&name=${encodeURIComponent(manager.name)}' target='_blank' rel='noopener noreferrer' aria-label='Открыть TODO для ${manager.name}'>добавить задачу</a>`
      : "";

  return `${manager.email}${todoLink}`;
}

export function localizeTaskStatus(status: string) {
  switch (status) {
    case "Pending":
      return "Ожидает";
    case "InProgress":
      return "В работе";
    case "Blocked":
      return "Заблокирована";
    case "Completed":
      return "Завершена";
    case "Archived":
      return "В архиве";
    default:
      return status;
  }
}

export function localizeTaskPriority(priority: string) {
  switch (priority) {
    case "Low":
      return "Низкий";
    case "Middle":
      return "Средний";
    case "High":
      return "Высокий";
    default:
      return priority;
  }
}

function renderEventContent(event: ClientEvent, todoServiceUrl: string) {
  const data = event.eventData;

  if (event.eventType === "DocumentLink") {
    return (
      <p className="mb-0">
        <a href={typeof data.url === "string" ? data.url : "#"}>
          {typeof data.text === "string" ? data.text : "Документ"}
        </a>
      </p>
    );
  }

  if (event.eventType === "Task") {
    const subject = typeof data.subject === "string" ? data.subject : "Задача";
    const publicId =
      typeof data.public_id === "string" ? data.public_id : undefined;
    const assignee =
      data.assignee && typeof data.assignee === "object"
        ? (data.assignee as Record<string, unknown>)
        : undefined;

    return (
      <>
        <p>
          {todoServiceUrl && publicId ? (
            <a
              href={`${todoServiceUrl}?public_id=${encodeURIComponent(publicId)}`}
              target="_blank"
              rel="noopener noreferrer"
            >
              <strong>{subject}</strong>
            </a>
          ) : (
            <strong>{subject}</strong>
          )}
        </p>
        {typeof data.status === "string" ? (
          <p>
            <span className="text-muted">Статус:</span>{" "}
            {localizeTaskStatus(data.status)}
          </p>
        ) : null}
        {typeof data.track === "string" ? (
          <p>
            <span className="text-muted">Трек:</span> {data.track}
          </p>
        ) : null}
        {typeof data.priority === "string" ? (
          <p>
            <span className="text-muted">Приоритет:</span>{" "}
            {localizeTaskPriority(data.priority)}
          </p>
        ) : null}
        {assignee ? (
          <p>
            <span className="text-muted">Исполнитель:</span>{" "}
            {typeof assignee.name === "string" ? assignee.name : "—"}
          </p>
        ) : null}
        {typeof data.text === "string" ? (
          <div dangerouslySetInnerHTML={{ __html: data.text }} />
        ) : null}
      </>
    );
  }

  return (
    <>
      {typeof data.subject === "string" ? (
        <p>
          <strong>{data.subject}</strong>
        </p>
      ) : null}
      {typeof data.text === "string" ? (
        <div dangerouslySetInnerHTML={{ __html: data.text }} />
      ) : null}
    </>
  );
}

function renderEventTypeBadge(eventType: string) {
  switch (eventType) {
    case "Comment":
      return (
        <span className="badge bg-primary bg-opacity-10 text-primary ms-2">
          комментарий
        </span>
      );
    case "DocumentLink":
      return (
        <span className="badge bg-warning bg-opacity-10 text-warning ms-2">
          вложение
        </span>
      );
    case "Call":
      return (
        <span className="badge bg-success bg-opacity-10 text-success ms-2">
          звонок
        </span>
      );
    case "Email":
      return (
        <span className="badge bg-info bg-opacity-10 text-info ms-2">
          рассылка
        </span>
      );
    case "Reply":
      return (
        <span className="badge bg-info bg-opacity-10 text-info ms-2">
          ответ
        </span>
      );
    case "Task":
      return (
        <span className="badge bg-secondary bg-opacity-10 text-secondary ms-2">
          задача
        </span>
      );
    case "Unsubscribed":
      return (
        <span className="badge bg-info bg-opacity-10 text-info ms-2">
          отписался
        </span>
      );
    default:
      return (
        <span className="badge bg-secondary bg-opacity-10 text-secondary ms-2">
          другое
        </span>
      );
  }
}

export function ClientBootstrap() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить React-оболочку CRM.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local CRM menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const [clientState, setClientState] = useState<ClientState>({
    status: "loading",
  });
  const [subject, setSubject] = useState("");
  const [message, setMessage] = useState("");
  const [eventType, setEventType] = useState("Comment");
  const [editableFields, setEditableFields] = useState<EditableFieldRow[]>([]);
  const [commentErrors, setCommentErrors] = useState<Record<string, string>>(
    {},
  );
  const [saveErrors, setSaveErrors] = useState<Record<string, string>>({});
  const [attachmentErrors, setAttachmentErrors] = useState<
    Record<string, string>
  >({});
  const [isCommentSubmitting, setIsCommentSubmitting] = useState(false);
  const [isSaveSubmitting, setIsSaveSubmitting] = useState(false);
  const [isAttachmentSubmitting, setIsAttachmentSubmitting] = useState(false);
  const [clientFormVersion, setClientFormVersion] = useState(0);
  const fileBrowserMounted = useRef(false);

  const loadClientData = async (clientId: number) => {
    const data = await fetchClientDetails(clientId);
    setClientState({ status: "ready", data });
    setEditableFields([
      ...data.importantFields.map((field, index) => ({
        id: `important-${index}`,
        label: field.label,
        value: field.value ?? "",
        readOnly: true,
      })),
      ...data.otherFields.map((field, index) => ({
        id: `other-${index}`,
        label: field.label,
        value: field.value ?? "",
        readOnly: false,
      })),
    ]);
    setClientFormVersion((current) => current + 1);
  };

  useEffect(() => {
    let active = true;

    const clientId = parseClientIdFromLocation();

    void fetchClientDetails(clientId)
      .then((data) => {
        if (!active) {
          return;
        }

        setClientState({ status: "ready", data });
        setEditableFields([
          ...data.importantFields.map((field, index) => ({
            id: `important-${index}`,
            label: field.label,
            value: field.value ?? "",
            readOnly: true,
          })),
          ...data.otherFields.map((field, index) => ({
            id: `other-${index}`,
            label: field.label,
            value: field.value ?? "",
            readOnly: false,
          })),
        ]);
        setClientFormVersion((current) => current + 1);
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setClientState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу клиента.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (clientState.status !== "ready") {
      return;
    }

    const bootstrap = window.bootstrap;
    const Popover = bootstrap?.Popover;
    if (!Popover) {
      return;
    }

    const elements = Array.from(
      document.querySelectorAll("[data-client-popover]"),
    );
    const popovers = elements.map((element) => new Popover(element));

    return () => {
      popovers.forEach((popover) => popover.dispose?.());
    };
  }, [clientState]);

  useEffect(() => {
    if (clientState.status !== "ready" || fileBrowserMounted.current) {
      return;
    }

    const target = document.getElementById("file-browser-root");
    if (!target) {
      return;
    }

    const script = document.createElement("script");
    script.src = `${clientState.data.filesServiceUrl}/assets/filebrowser.js`;
    script.async = true;
    script.onload = () => {
      if (window.mountFileBrowser) {
        window.mountFileBrowser("#file-browser-root", "", {
          baseUrl: clientState.data.filesServiceUrl,
          historyMode: "disabled",
        });
        fileBrowserMounted.current = true;
      }
    };
    document.body.appendChild(script);

    return () => {
      script.remove();
    };
  }, [clientState]);

  const renderedMessage = useMemo(() => {
    return renderMarkdownToHtml(message);
  }, [message]);

  const addEditableField = () => {
    setEditableFields((current) => [
      ...current,
      {
        id: `new-${current.length}-${Date.now()}`,
        label: "",
        value: "",
        readOnly: false,
      },
    ]);
  };

  const updateEditableField = (
    id: string,
    patch: Partial<Pick<EditableFieldRow, "label" | "value">>,
  ) => {
    setEditableFields((current) =>
      current.map((field) =>
        field.id === id ? { ...field, ...patch } : field,
      ),
    );
  };

  const removeEditableField = (id: string) => {
    setEditableFields((current) => current.filter((field) => field.id !== id));
  };

  if (shellState.status === "loading" || clientState.status === "loading") {
    return null;
  }

  if (shellState.status === "error") {
    return <CrmShellFatalState message={shellState.message} />;
  }

  if (clientState.status === "error") {
    return <CrmShellFatalState message={clientState.message} />;
  }

  const client = clientState.data.client;

  async function handleCommentSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsCommentSubmitting(true);
    setCommentErrors({});

    const body = new URLSearchParams();
    body.set("subject", subject);
    body.set("message", renderedMessage);
    body.set("event_type", eventType);

    try {
      const result = await postForm(`/client/${client.id}/comment`, body);
      window.showFlashMessage?.(result.message, "success");
      setSubject("");
      setMessage("");
      setEventType("Comment");
      await loadClientData(client.id);
    } catch (error) {
      if (isApiMutationError(error)) {
        setCommentErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to submit client comment.", error);
        window.showFlashMessage?.("Не удалось сохранить событие.", "danger");
      }
    } finally {
      setIsCommentSubmitting(false);
    }
  }

  async function handleSaveSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    setIsSaveSubmitting(true);
    setSaveErrors({});

    const body = new URLSearchParams();
    const formData = new FormData(form);

    for (const [key, value] of formData.entries()) {
      body.append(key, String(value));
    }

    try {
      const result = await postForm(`/client/${client.id}/save`, body);
      window.showFlashMessage?.(result.message, "success");
      window.bootstrap?.Modal.getOrCreateInstance("#clientModal", {}).hide();
      await loadClientData(client.id);
    } catch (error) {
      if (isApiMutationError(error)) {
        setSaveErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to save client.", error);
        window.showFlashMessage?.("Не удалось сохранить клиента.", "danger");
      }
    } finally {
      setIsSaveSubmitting(false);
    }
  }

  async function handleAttachmentSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    setIsAttachmentSubmitting(true);
    setAttachmentErrors({});

    const body = new URLSearchParams();
    const formData = new FormData(form);

    for (const [key, value] of formData.entries()) {
      body.append(key, String(value));
    }

    try {
      const result = await postForm(`/client/${client.id}/attachment`, body);
      window.showFlashMessage?.(result.message, "success");
      window.bootstrap?.Modal.getOrCreateInstance(
        "#attachmentModal",
        {},
      ).hide();
      form.reset();
      await loadClientData(client.id);
    } catch (error) {
      if (isApiMutationError(error)) {
        setAttachmentErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to attach client document.", error);
        window.showFlashMessage?.("Не удалось добавить вложение.", "danger");
      }
    } finally {
      setIsAttachmentSubmitting(false);
    }
  }

  function clearCommentError(field: string) {
    setCommentErrors((current) => ({ ...current, [field]: "" }));
  }

  function clearSaveError(field: string) {
    setSaveErrors((current) => ({ ...current, [field]: "" }));
  }

  function clearAttachmentError(field: string) {
    setAttachmentErrors((current) => ({ ...current, [field]: "" }));
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
        <div className="row my-2">
          <div className="col-sm">
            <span className="fw-bold">{client.name}</span>
          </div>
        </div>
        <div className="row crm-client-page">
          <div className="col-md-5 my-1 crm-client-summary">
            <div className="row">
              <div className="col">
                <a
                  className="link-offset-2 link-offset-2-hover link-underline link-underline-opacity-0 link-underline-opacity-75-hover"
                  href="#clientModal"
                  data-bs-toggle="modal"
                >
                  {client.email ?? "Электронный адрес"}
                </a>
              </div>
              <div className="col text-end">
                <div className="dropdown">
                  <a
                    className="dropdown-toggle"
                    href="#"
                    role="button"
                    id="orderActionsLink"
                    data-bs-toggle="dropdown"
                    aria-expanded="false"
                  >
                    Действия
                  </a>
                  <ul
                    className="dropdown-menu bg-light"
                    aria-labelledby="orderActionsLink"
                  >
                    <li className="dropdown-item">
                      <i className="bi bi-file" />{" "}
                      <a
                        href="#"
                        data-bs-toggle="modal"
                        data-bs-target="#attachmentModal"
                      >
                        вложение
                      </a>
                    </li>
                  </ul>
                </div>
              </div>
            </div>
            <div className="row">
              <div className="col">
                <a
                  className="link-offset-2 link-offset-2-hover link-underline link-underline-opacity-0 link-underline-opacity-75-hover"
                  href="#clientModal"
                  data-bs-toggle="modal"
                >
                  {client.phone ?? "Телефон"}
                </a>
              </div>
            </div>
            {clientState.data.importantFields.map((field) => (
              <div className="row" key={field.label}>
                <div className="col overflow-hidden">
                  <a
                    className="link-offset-2 link-offset-2-hover link-underline link-underline-opacity-0 link-underline-opacity-75-hover"
                    href="#clientModal"
                    data-bs-toggle="modal"
                  >
                    {field.value ?? field.label}
                  </a>
                </div>
              </div>
            ))}
            {clientState.data.otherFields.length > 0 ? (
              <>
                <div className="row">
                  <div className="col">
                    <a
                      className="text-nowrap"
                      data-bs-toggle="collapse"
                      href="#client-fields"
                      aria-expanded="false"
                      aria-controls="client-fields"
                    >
                      Подробнее <i className="bi bi-caret-down-fill" />
                    </a>
                  </div>
                </div>
                <div className="collapse" id="client-fields">
                  {clientState.data.otherFields.map((field) => (
                    <div key={field.label}>
                      <div className="row">
                        <div className="col overflow-hidden">
                          <a
                            className="fw-bold link-secondary link-offset-2 link-offset-2-hover link-underline link-underline-opacity-0 link-underline-opacity-75-hover"
                            href="#clientModal"
                            data-bs-toggle="modal"
                          >
                            {field.label}
                          </a>
                        </div>
                      </div>
                      <div className="row border-bottom">
                        <div className="col text-truncate toggle-text-truncate">
                          {field.value ?? "—"}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </>
            ) : null}
            <div className="row">
              <div className="col">
                <span className="fw-bold">Менеджеры:</span>{" "}
                {clientState.data.managers.length > 0 ? (
                  clientState.data.managers.map((manager) => (
                    <a
                      key={manager.id}
                      className="text-nowrap link-offset-2 link-offset-2-hover link-underline link-underline-opacity-0 link-underline-opacity-75-hover link-dark me-2"
                      role="button"
                      tabIndex={0}
                      data-bs-trigger="focus"
                      data-bs-toggle="popover"
                      data-client-popover="true"
                      title={manager.name}
                      data-bs-content={renderManagerPopover(
                        manager,
                        clientState.data.todoServiceUrl,
                      )}
                      aria-label={manager.name}
                      data-bs-html="true"
                    >
                      <i className="bi bi-person-circle" />
                      &nbsp;{manager.name}
                    </a>
                  ))
                ) : (
                  <>—</>
                )}
              </div>
            </div>
            {clientState.data.documents.length > 0 ? (
              <div className="row">
                <div className="col">
                  <span className="fw-bold">Документы:</span>
                  <ul>
                    {clientState.data.documents.map((document) => (
                      <li key={document.id}>
                        <a
                          className="link-warning link-offset-2 link-offset-2-hover link-underline link-underline-opacity-0 link-underline-opacity-75-hover"
                          href={
                            typeof document.eventData.url === "string"
                              ? document.eventData.url
                              : "#"
                          }
                        >
                          {typeof document.eventData.text === "string"
                            ? document.eventData.text
                            : "Документ"}
                        </a>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            ) : null}
          </div>
          <div className="col-md my-1 crm-client-events">
            <form onSubmit={(event) => void handleCommentSubmit(event)}>
              <div className="row">
                <div className="col-sm">
                  <div className="row">
                    <div className="col">
                      <input
                        id="event-form-subject"
                        type={eventType === "Email" ? "text" : "hidden"}
                        name="subject"
                        className={
                          commentErrors.subject
                            ? "form-control form-control-sm mb-1 is-invalid"
                            : "form-control form-control-sm mb-1"
                        }
                        placeholder="Тема"
                        value={subject}
                        onChange={(event) => {
                          setSubject(event.target.value);
                          clearCommentError("subject");
                        }}
                      />
                      {commentErrors.subject ? (
                        <div className="invalid-feedback d-block">
                          {commentErrors.subject}
                        </div>
                      ) : null}
                    </div>
                  </div>
                  <MarkdownComposer
                    id="message-input"
                    className="mb-1"
                    value={message}
                    onChange={(nextMessage) => {
                      setMessage(nextMessage);
                      clearCommentError("message");
                    }}
                    rows={10}
                    required
                    autoFocus
                    placeholder="Содержание в формате markdown"
                    textareaClassName={
                      commentErrors.message ? "is-invalid" : undefined
                    }
                    previewClassName="crm-markdown-preview"
                    editorLabel="Маркдаун"
                    previewLabel="Превью"
                    fileBrowserLabel="Файлы"
                    emptyPreviewLabel="Пока нечего показывать."
                    fileBrowser={
                      clientState.data.filesServiceUrl
                        ? {
                            baseUrl: clientState.data.filesServiceUrl,
                            helpText:
                              "Загрузите или найдите файл, скопируйте ссылку и вставьте её в markdown как ссылку или изображение.",
                          }
                        : undefined
                    }
                  />
                  {commentErrors.message ? (
                    <div className="invalid-feedback d-block">
                      {commentErrors.message}
                    </div>
                  ) : null}
                  <input
                    type="hidden"
                    id="message"
                    name="message"
                    required
                    value={renderedMessage}
                    readOnly
                  />
                </div>
                <div className="col-sm-auto">
                  <div className="row">
                    <div className="col">
                      <select
                        name="event_type"
                        id="event-form-event-type"
                        className="form-select form-select-sm"
                        aria-label="Select event type"
                        required
                        value={eventType}
                        onChange={(event) => {
                          setEventType(event.target.value);
                          clearCommentError("event_type");
                        }}
                      >
                        <option value="Comment">Комментарий</option>
                        <option value="Email">Email</option>
                      </select>
                    </div>
                  </div>
                  <div className="row">
                    <div className="col">
                      <button
                        className="btn btn-primary btn-sm my-1 w-100"
                        type="submit"
                        disabled={isCommentSubmitting}
                      >
                        Сохранить
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </form>
            <div id="events">
              {clientState.data.events.map((event) => (
                <div
                  className="card border-start border mb-1 shadow-sm event"
                  key={event.id}
                >
                  <div className="card-body">
                    <div className="d-flex justify-content-between small text-muted mb-2">
                      <span>{event.createdAt}</span>
                      <span>
                        <a
                          className="text-nowrap link-offset-2 link-offset-2-hover link-underline link-underline-opacity-0 link-underline-opacity-75-hover link-dark"
                          role="button"
                          tabIndex={0}
                          data-bs-trigger="focus"
                          data-bs-toggle="popover"
                          data-client-popover="true"
                          title={event.manager.name}
                          data-bs-content={renderManagerPopover(
                            event.manager,
                            clientState.data.todoServiceUrl,
                          )}
                          aria-label={event.manager.name}
                          data-bs-html="true"
                        >
                          <i className="bi bi-person-circle" />
                          &nbsp;{event.manager.name}
                        </a>
                        {renderEventTypeBadge(event.eventType)}
                      </span>
                    </div>
                    {renderEventContent(event, clientState.data.todoServiceUrl)}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

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
                Изменить клиента
              </h1>
              <button
                type="button"
                className="btn-close"
                data-bs-dismiss="modal"
                aria-label="Close"
              />
            </div>
            <div className="modal-body">
              <form
                key={`save-${clientFormVersion}`}
                onSubmit={(event) => void handleSaveSubmit(event)}
              >
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
                        saveErrors.name
                          ? "form-control is-invalid"
                          : "form-control"
                      }
                      id="clientModalName"
                      defaultValue={client.name}
                      placeholder="Имя"
                      required
                      onChange={() => clearSaveError("name")}
                    />
                    {saveErrors.name ? (
                      <div className="invalid-feedback">{saveErrors.name}</div>
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
                        saveErrors.email
                          ? "form-control is-invalid"
                          : "form-control"
                      }
                      id="clientModalEmail"
                      defaultValue={client.email ?? ""}
                      placeholder="Электронный адрес"
                      onChange={() => clearSaveError("email")}
                    />
                    {saveErrors.email ? (
                      <div className="invalid-feedback">{saveErrors.email}</div>
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
                        saveErrors.phone
                          ? "form-control is-invalid"
                          : "form-control"
                      }
                      id="clientModalPhone"
                      defaultValue={client.phone ?? ""}
                      placeholder="Телефон"
                      onChange={() => clearSaveError("phone")}
                    />
                    {saveErrors.phone ? (
                      <div className="invalid-feedback">{saveErrors.phone}</div>
                    ) : null}
                  </div>
                </div>
                <div id="custom-fields">
                  {editableFields.map((field) => (
                    <div className="row mb-3" key={field.id}>
                      <div className="col">
                        <input
                          list="available-custom-fields"
                          type="text"
                          className="form-control"
                          value={field.label}
                          name="field"
                          placeholder="Поле"
                          required
                          readOnly={field.readOnly}
                          onChange={(event) =>
                            updateEditableField(field.id, {
                              label: event.target.value,
                            })
                          }
                        />
                      </div>
                      <div className="col">
                        <input
                          type="text"
                          className="form-control"
                          value={field.value}
                          name="value"
                          placeholder="Значение"
                          onChange={(event) =>
                            updateEditableField(field.id, {
                              value: event.target.value,
                            })
                          }
                        />
                      </div>
                      <div className="col-auto">
                        <button
                          type="button"
                          className="btn btn-danger btn-sm"
                          onClick={() => removeEditableField(field.id)}
                        >
                          <i className="bi bi-slash-circle" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
                <div className="row mb-3">
                  <div className="col">
                    <button
                      className="btn btn-primary"
                      type="submit"
                      disabled={isSaveSubmitting}
                    >
                      Сохранить
                    </button>
                  </div>
                  <div className="col-auto">
                    <button
                      type="button"
                      className="btn btn-primary btn-sm"
                      onClick={addEditableField}
                    >
                      <i className="bi bi-plus" />
                    </button>
                  </div>
                </div>
              </form>
            </div>
          </div>
        </div>
      </div>

      <div
        className="modal fade"
        id="attachmentModal"
        tabIndex={-1}
        aria-labelledby="attachmentModalLabel"
        aria-hidden="true"
      >
        <div className="modal-dialog modal-lg">
          <div className="modal-content">
            <div className="modal-header">
              <h1 className="modal-title fs-5" id="attachmentModalLabel">
                Добавить вложение
              </h1>
              <button
                type="button"
                className="btn-close"
                data-bs-dismiss="modal"
                aria-label="Close"
              />
            </div>
            <div className="modal-body">
              <div className="border-bottom" id="file-browser-root" />
              <form
                key={`attachment-${clientFormVersion}`}
                onSubmit={(event) => void handleAttachmentSubmit(event)}
              >
                <div className="row mb-3">
                  <div className="col-md">
                    <input
                      type="text"
                      name="text"
                      className={
                        attachmentErrors.text
                          ? "form-control my-1 is-invalid"
                          : "form-control my-1"
                      }
                      placeholder="Название"
                      required
                      onChange={() => clearAttachmentError("text")}
                    />
                    {attachmentErrors.text ? (
                      <div className="invalid-feedback d-block">
                        {attachmentErrors.text}
                      </div>
                    ) : null}
                    <div className="form-text text-muted">
                      Название документа
                    </div>
                  </div>
                  <div className="col-md">
                    <input
                      type="url"
                      name="url"
                      className={
                        attachmentErrors.url
                          ? "form-control my-1 is-invalid"
                          : "form-control my-1"
                      }
                      placeholder="https://example.com/"
                      required
                      onChange={() => clearAttachmentError("url")}
                    />
                    {attachmentErrors.url ? (
                      <div className="invalid-feedback d-block">
                        {attachmentErrors.url}
                      </div>
                    ) : null}
                    <div className="form-text text-muted">
                      Ссылка на документ
                    </div>
                  </div>
                </div>
                <div className="row mb-3">
                  <div className="col">
                    <button
                      className="btn btn-primary"
                      type="submit"
                      disabled={isAttachmentSubmitting}
                    >
                      Сохранить
                    </button>
                  </div>
                </div>
              </form>
            </div>
          </div>
        </div>
      </div>

      <datalist id="available-custom-fields">
        {clientState.data.availableFields.map((field) => (
          <option value={field} key={field} />
        ))}
      </datalist>
    </CrmShell>
  );
}
