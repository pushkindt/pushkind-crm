import {
  browserLocation,
  fetchHubMenuItems as fetchSharedHubMenuItems,
  fetchJson as fetchSharedJson,
  fetchNoAccessData as fetchSharedNoAccessData,
  fetchShellData as fetchSharedShellData,
} from "@pushkind/frontend-shell/shellApi";
import {
  isRecord,
  parseStringMap,
  readBoolean,
  readNullableNumberArray,
  readNumber,
  readOptionalString,
  readString,
} from "@pushkind/frontend-shell/json";

export { browserLocation };
import {
  type ApiFieldError,
  type ApiMutationError,
  type ApiMutationSuccess,
  isApiMutationError,
  postEmpty,
  postForm,
  postMultipartForm,
  toFieldErrorMap,
} from "@pushkind/frontend-shell/mutations";
import type {
  AuthUserSearchItem,
  ClientDetails,
  ClientDirectoryData,
  ClientEvent,
  ClientFieldDisplay,
  ClientListItem,
  ImportantFieldSettingsData,
  Manager,
  ManagerModalData,
  ManagersData,
  ManagerWithClients,
  NoAccessData,
  ShellData,
  UserMenuItem,
} from "./models";

function readOptionalStringArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (value == null) {
    return undefined;
  }
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`Invalid API response: expected string[] at ${key}.`);
  }

  return value;
}

function parseClientListItems(payload: unknown): ClientListItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid client list payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid client item payload.");
    }

    return {
      id: readNumber(item, "id"),
      publicId: readOptionalString(item, "public_id"),
      name: readString(item, "name"),
      email: readOptionalString(item, "email"),
      phone: readOptionalString(item, "phone"),
      fieldBadges:
        readOptionalStringArray(item, "field_badges") ??
        Object.values(parseStringMap(item.fields)).slice(0, 8),
    };
  });
}

function parseClientDirectoryData(payload: unknown): ClientDirectoryData {
  if (!isRecord(payload) || !isRecord(payload.clients)) {
    throw new Error("Invalid client directory payload.");
  }

  return {
    searchQuery: readOptionalString(payload, "search_query"),
    clients: {
      items: parseClientListItems(payload.clients.items),
      page: readNumber(payload.clients, "page"),
      pages: readNullableNumberArray(payload.clients, "pages"),
    },
  };
}

function parseManager(item: unknown): Manager {
  if (!isRecord(item)) {
    throw new Error("Invalid manager payload.");
  }

  return {
    id: readNumber(item, "id"),
    name: readString(item, "name"),
    email: readString(item, "email"),
    isUser: readBoolean(item, "is_user"),
  };
}

function parseClientFieldDisplay(item: unknown): ClientFieldDisplay {
  if (!isRecord(item)) {
    throw new Error("Invalid client field payload.");
  }

  return {
    label: readString(item, "label"),
    value: readOptionalString(item, "value"),
  };
}

function parseEventData(item: unknown): Record<string, unknown> {
  if (!isRecord(item)) {
    return {};
  }

  return item;
}

function parseClientEvent(item: unknown): ClientEvent {
  if (!isRecord(item)) {
    throw new Error("Invalid client event payload.");
  }

  return {
    id: readNumber(item, "id"),
    eventType: readString(item, "event_type"),
    eventData: parseEventData(item.event_data),
    createdAt: readString(item, "created_at"),
    manager: parseManager(item.manager),
  };
}

function parseClientDetails(payload: unknown): ClientDetails {
  if (!isRecord(payload) || !isRecord(payload.client)) {
    throw new Error("Invalid client details payload.");
  }

  return {
    client: {
      id: readNumber(payload.client, "id"),
      publicId: readOptionalString(payload.client, "public_id"),
      name: readString(payload.client, "name"),
      email: readOptionalString(payload.client, "email"),
      phone: readOptionalString(payload.client, "phone"),
      fields: parseStringMap(payload.client.fields),
    },
    managers: Array.isArray(payload.managers)
      ? payload.managers.map(parseManager)
      : [],
    events: Array.isArray(payload.events)
      ? payload.events.map(parseClientEvent)
      : [],
    documents: Array.isArray(payload.documents)
      ? payload.documents.map(parseClientEvent)
      : [],
    availableFields: Array.isArray(payload.available_fields)
      ? payload.available_fields.filter(
          (item): item is string => typeof item === "string",
        )
      : [],
    importantFields: Array.isArray(payload.important_fields)
      ? payload.important_fields.map(parseClientFieldDisplay)
      : [],
    otherFields: Array.isArray(payload.other_fields)
      ? payload.other_fields.map(parseClientFieldDisplay)
      : [],
    totalEvents: readNumber(payload, "total_events"),
    todoServiceUrl: readString(payload, "todo_service_url"),
    filesServiceUrl: readString(payload, "files_service_url"),
  };
}

function parseManagersData(payload: unknown): ManagersData {
  if (!isRecord(payload) || !Array.isArray(payload.managers)) {
    throw new Error("Invalid managers payload.");
  }

  return {
    managers: payload.managers.map((item) => {
      if (!isRecord(item)) {
        throw new Error("Invalid manager-with-clients payload.");
      }

      return {
        manager: parseManager(item.manager),
        clients: parseClientListItems(item.clients),
      } satisfies ManagerWithClients;
    }),
  };
}

function parseManagerModalData(payload: unknown): ManagerModalData {
  if (!isRecord(payload)) {
    throw new Error("Invalid manager modal payload.");
  }

  return {
    manager: parseManager(payload.manager),
    clients: parseClientListItems(payload.clients),
  };
}

function parseImportantFieldSettingsData(
  payload: unknown,
): ImportantFieldSettingsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid important-field settings payload.");
  }

  return {
    fieldsText: readString(payload, "fields_text"),
  };
}

function parseAuthUsers(payload: unknown): AuthUserSearchItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid auth users payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid auth user payload.");
    }

    return {
      sub: readString(item, "sub"),
      name: readString(item, "name"),
      email: readString(item, "email"),
    };
  });
}

function withBaseUrl(baseUrl: string, path: string) {
  return new URL(path, baseUrl).toString();
}

async function fetchJson(url: string) {
  return fetchSharedJson(url, {
    unauthorizedMessage: "Недостаточно прав для доступа к CRM.",
  });
}

export {
  isApiMutationError,
  postEmpty,
  postForm,
  postMultipartForm,
  toFieldErrorMap,
};

export async function fetchShellData(): Promise<ShellData> {
  return fetchSharedShellData<ShellData>(
    "/api/v1/iam",
    "Недостаточно прав для доступа к CRM.",
  );
}

export async function fetchNoAccessData(): Promise<NoAccessData> {
  const query = window.location.search;
  return fetchSharedNoAccessData<NoAccessData>(
    query ? `/api/v1/no-access${query}` : "/api/v1/no-access",
    "Недостаточно прав для доступа к CRM.",
  );
}

export async function fetchClientDirectoryData(
  searchParams: URLSearchParams,
): Promise<ClientDirectoryData> {
  const query = searchParams.toString();
  const payload = await fetchJson(
    query ? `/api/v1/client-directory?${query}` : "/api/v1/client-directory",
  );
  return parseClientDirectoryData(payload);
}

export async function fetchClientDetails(
  clientId: number,
): Promise<ClientDetails> {
  const payload = await fetchJson(`/api/v1/clients/${clientId}`);
  return parseClientDetails(payload);
}

export async function fetchManagersData(): Promise<ManagersData> {
  const payload = await fetchJson("/api/v1/managers");
  return parseManagersData(payload);
}

export async function fetchClientsData(
  search: string,
): Promise<ClientListItem[]> {
  const payload = await fetchJson(
    `/api/v1/clients?search=${encodeURIComponent(search)}&page=1`,
  );
  return parseClientListItems(payload);
}

export async function fetchManagerModalData(
  managerId: number,
): Promise<ManagerModalData> {
  const payload = await fetchJson(`/api/v1/managers/${managerId}`);
  return parseManagerModalData(payload);
}

export async function fetchImportantFieldSettingsData(): Promise<ImportantFieldSettingsData> {
  const payload = await fetchJson("/api/v1/important-fields");
  return parseImportantFieldSettingsData(payload);
}

export async function fetchAuthUsers(
  authBaseUrl: string,
  query: string,
): Promise<AuthUserSearchItem[]> {
  const payload = await fetchJson(
    withBaseUrl(
      authBaseUrl,
      `/api/v1/users?page=1&role=crm_manager&query=${encodeURIComponent(query)}`,
    ),
  );

  return parseAuthUsers(payload).slice(0, 20);
}

export async function fetchHubMenuItems(
  authBaseUrl: string,
  hubId: number,
): Promise<UserMenuItem[]> {
  return fetchSharedHubMenuItems<UserMenuItem>(
    withBaseUrl(authBaseUrl, `/api/v1/hubs/${hubId}/menu-items`),
    "Недостаточно прав для доступа к CRM.",
  );
}
