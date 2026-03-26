import type {
  ClientDetails,
  ClientEvent,
  ClientFieldDisplay,
  AuthUserSearchItem,
  Manager,
  ManagerModalData,
  ManagersData,
  ManagerWithClients,
  ClientListItem,
  DashboardData,
  NavigationItem,
  ShellData,
  SettingsData,
  UserMenuItem,
} from "./models";

export interface ApiFieldError {
  field: string;
  message: string;
}

export interface ApiMutationSuccess {
  message: string;
  redirect_to: string | null;
}

export interface ApiMutationError {
  message: string;
  field_errors: ApiFieldError[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string at ${key}.`);
  }

  return value;
}

function readNumber(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Invalid API response: expected number at ${key}.`);
  }

  return value;
}

function readStringArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`Invalid API response: expected string[] at ${key}.`);
  }

  return value;
}

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

function readNullableNumberArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (
    !Array.isArray(value) ||
    value.some((item) => item !== null && typeof item !== "number")
  ) {
    throw new Error(
      `Invalid API response: expected (number|null)[] at ${key}.`,
    );
  }

  return value;
}

function readOptionalString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (value == null) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string at ${key}.`);
  }

  return value;
}

function readBoolean(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "boolean") {
    throw new Error(`Invalid API response: expected boolean at ${key}.`);
  }

  return value;
}

function parseNavigationItems(payload: unknown): NavigationItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid navigation payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid navigation item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
}

function parseShellData(payload: unknown): ShellData {
  if (!isRecord(payload) || !isRecord(payload.current_user)) {
    throw new Error("Invalid shell payload.");
  }

  return {
    currentUser: {
      email: readString(payload.current_user, "email"),
      name: readString(payload.current_user, "name"),
      hubId: readNumber(payload.current_user, "hub_id"),
      roles: readStringArray(payload.current_user, "roles"),
    },
    homeUrl: readString(payload, "home_url"),
    navigation: parseNavigationItems(payload.navigation),
    localMenuItems: parseNavigationItems(payload.local_menu_items),
  };
}

function parseMenuItems(payload: unknown): UserMenuItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid auth menu payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid auth menu item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
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

function parseDashboardData(payload: unknown): DashboardData {
  if (!isRecord(payload) || !isRecord(payload.clients)) {
    throw new Error("Invalid dashboard payload.");
  }

  return {
    searchQuery: readOptionalString(payload, "search_query"),
    canAddClient: readBoolean(payload, "can_add_client"),
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

function parseStringMap(value: unknown) {
  if (!isRecord(value)) {
    return {};
  }

  return Object.fromEntries(
    Object.entries(value).filter((entry): entry is [string, string] => {
      return typeof entry[1] === "string";
    }),
  );
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

function parseSettingsData(payload: unknown): SettingsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid settings payload.");
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
  const response = await fetch(url, {
    headers: {
      Accept: "application/json",
    },
    cache: "no-store",
    credentials: "include",
  });

  if (!response.ok) {
    if (response.status === 401) {
      throw new Error("Недостаточно прав для доступа к CRM.");
    }

    throw new Error(`Request failed with status ${response.status}.`);
  }

  return response.json();
}

function isJsonResponse(response: Response): boolean {
  return (
    response.headers.get("content-type")?.includes("application/json") ?? false
  );
}

export const browserLocation = {
  assign(url: string) {
    window.location.assign(url);
  },
};

function handleAuthRedirectResponse(response: Response): never {
  browserLocation.assign(response.url);
  throw new Error("Сессия истекла. Выполняется переход на страницу входа.");
}

function ensureMutationResponseIsNotAuthRedirect(response: Response) {
  if (response.redirected && !isJsonResponse(response)) {
    handleAuthRedirectResponse(response);
  }
}

async function readJsonResponse<T>(response: Response, endpoint: string) {
  if (!isJsonResponse(response)) {
    throw new Error(
      `Expected JSON response from ${endpoint} with status ${response.status}.`,
    );
  }

  return (await response.json()) as T;
}

export function toFieldErrorMap(
  error: ApiMutationError,
): Record<string, string> {
  return Object.fromEntries(
    error.field_errors.map((fieldError) => [
      fieldError.field,
      fieldError.message,
    ]),
  );
}

export function isApiMutationError(error: unknown): error is ApiMutationError {
  if (!isRecord(error)) {
    return false;
  }

  return (
    typeof error.message === "string" &&
    Array.isArray(error.field_errors) &&
    error.field_errors.every((fieldError) => {
      return (
        isRecord(fieldError) &&
        typeof fieldError.field === "string" &&
        typeof fieldError.message === "string"
      );
    })
  );
}

export async function postForm(
  endpoint: string,
  body: URLSearchParams,
): Promise<ApiMutationSuccess> {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
    },
    credentials: "include",
    body: body.toString(),
  });

  ensureMutationResponseIsNotAuthRedirect(response);

  const payload = (await readJsonResponse(response, endpoint)) as
    | ApiMutationSuccess
    | ApiMutationError;

  if (!response.ok) {
    throw payload as ApiMutationError;
  }

  return payload as ApiMutationSuccess;
}

export async function postMultipartForm(
  endpoint: string,
  body: FormData,
): Promise<ApiMutationSuccess> {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      Accept: "application/json",
    },
    credentials: "include",
    body,
  });

  ensureMutationResponseIsNotAuthRedirect(response);

  const payload = (await readJsonResponse(response, endpoint)) as
    | ApiMutationSuccess
    | ApiMutationError;

  if (!response.ok) {
    throw payload as ApiMutationError;
  }

  return payload as ApiMutationSuccess;
}

export async function postEmpty(endpoint: string): Promise<ApiMutationSuccess> {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      Accept: "application/json",
    },
    credentials: "include",
  });

  ensureMutationResponseIsNotAuthRedirect(response);

  const payload = (await readJsonResponse(response, endpoint)) as
    | ApiMutationSuccess
    | ApiMutationError;

  if (!response.ok) {
    throw payload as ApiMutationError;
  }

  return payload as ApiMutationSuccess;
}

export async function fetchShellData(): Promise<ShellData> {
  const payload = await fetchJson("/api/v1/iam");
  return parseShellData(payload);
}

export async function fetchDashboardData(
  searchParams: URLSearchParams,
): Promise<DashboardData> {
  const query = searchParams.toString();
  const payload = await fetchJson(
    query ? `/api/v1/dashboard?${query}` : "/api/v1/dashboard",
  );
  return parseDashboardData(payload);
}

export async function fetchClientDetails(
  clientId: number,
): Promise<ClientDetails> {
  const payload = await fetchJson(`/api/v1/client/${clientId}`);
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

export async function fetchSettingsData(): Promise<SettingsData> {
  const payload = await fetchJson("/api/v1/settings");
  return parseSettingsData(payload);
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
  const payload = await fetchJson(
    withBaseUrl(authBaseUrl, `/api/v1/hubs/${hubId}/menu-items`),
  );
  return parseMenuItems(payload);
}
