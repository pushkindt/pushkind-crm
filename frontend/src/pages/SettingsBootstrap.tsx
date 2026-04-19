import { useEffect, useState } from "react";
import type { FormEvent } from "react";

import { CrmShell } from "../components/CrmShell";
import { CrmShellFatalState } from "../components/CrmShellFatalState";
import {
  fetchHubMenuItems,
  fetchImportantFieldSettingsData,
  fetchShellData,
  isApiMutationError,
  postEmpty,
  postForm,
  toFieldErrorMap,
} from "../lib/api";
import type {
  ImportantFieldSettingsData,
  ShellData,
  UserMenuItem,
} from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

type SettingsState =
  | { status: "loading" }
  | { status: "ready"; data: ImportantFieldSettingsData }
  | { status: "error"; message: string };

export function SettingsBootstrap() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить React-оболочку CRM.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local CRM menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const [settingsState, setSettingsState] = useState<SettingsState>({
    status: "loading",
  });
  const [fieldsText, setFieldsText] = useState("");
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [isCleaning, setIsCleaning] = useState(false);

  useEffect(() => {
    let active = true;

    void fetchImportantFieldSettingsData()
      .then((data) => {
        if (!active) {
          return;
        }

        setSettingsState({ status: "ready", data });
        setFieldsText(data.fieldsText);
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setSettingsState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу настроек.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

  if (shellState.status === "loading" || settingsState.status === "loading") {
    return null;
  }

  if (shellState.status === "error") {
    return <CrmShellFatalState message={shellState.message} />;
  }

  if (settingsState.status === "error") {
    return <CrmShellFatalState message={settingsState.message} />;
  }

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);
    setFieldErrors({});

    const body = new URLSearchParams();
    body.set("fields", fieldsText);

    try {
      const result = await postForm("/important-fields", body);
      setSettingsState({ status: "ready", data: { fieldsText } });
      window.showFlashMessage?.(result.message, "success");
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to save important fields.", error);
        window.showFlashMessage?.(
          "Не удалось сохранить важные поля.",
          "danger",
        );
      }
    } finally {
      setIsSaving(false);
    }
  }

  async function handleCleanup() {
    if (!window.confirm("Удалить всех клиентов и связанные записи?")) {
      return;
    }

    setIsCleaning(true);

    try {
      const result = await postEmpty("/settings/cleanup");
      window.showFlashMessage?.(result.message, "success");
    } catch (error) {
      if (isApiMutationError(error)) {
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to clean up CRM clients.", error);
        window.showFlashMessage?.("Не удалось удалить клиентов.", "danger");
      }
    } finally {
      setIsCleaning(false);
    }
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
        <div className="row justify-content-center">
          <div className="col-lg-8">
            <div className="card shadow-sm">
              <div className="card-body">
                <h1 className="h4 mb-3">Важные поля клиента</h1>
                <p className="text-muted">
                  Укажите названия полей по одному на строку. Пустые строки
                  будут проигнорированы.
                </p>
                <form onSubmit={(event) => void handleSave(event)}>
                  <div className="mb-3">
                    <label
                      htmlFor="important-fields-textarea"
                      className="form-label"
                    >
                      Список полей
                    </label>
                    <textarea
                      className={
                        fieldErrors.fields
                          ? "form-control is-invalid"
                          : "form-control"
                      }
                      id="important-fields-textarea"
                      name="fields"
                      rows={10}
                      value={fieldsText}
                      onChange={(event) => {
                        setFieldsText(event.target.value);
                        setFieldErrors((current) => ({
                          ...current,
                          fields: "",
                          name: "",
                        }));
                      }}
                    />
                    {fieldErrors.fields || fieldErrors.name ? (
                      <div className="invalid-feedback">
                        {fieldErrors.fields || fieldErrors.name}
                      </div>
                    ) : null}
                  </div>
                  <button
                    type="submit"
                    className="btn btn-primary"
                    disabled={isSaving}
                  >
                    Сохранить
                  </button>
                </form>
                <hr className="my-4" />
                <div>
                  <button
                    type="button"
                    className="btn btn-outline-danger"
                    onClick={() => void handleCleanup()}
                    disabled={isCleaning}
                  >
                    Удалить клиентов
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </CrmShell>
  );
}
