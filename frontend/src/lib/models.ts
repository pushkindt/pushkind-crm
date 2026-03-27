export type NavigationItem = {
  name: string;
  url: string;
};

export type UserMenuItem = {
  name: string;
  url: string;
};

export type ShellData = {
  currentUser: {
    email: string;
    name: string;
    hubId: number;
    roles: string[];
  };
  homeUrl: string;
  navigation: NavigationItem[];
  localMenuItems: UserMenuItem[];
};

export type ClientListItem = {
  id: number;
  publicId?: string;
  name: string;
  email?: string;
  phone?: string;
  fieldBadges: string[];
};

export type PaginatedClientList = {
  items: ClientListItem[];
  pages: Array<number | null>;
  page: number;
};

export type ClientDirectoryData = {
  searchQuery?: string;
  clients: PaginatedClientList;
};

export type Manager = {
  id: number;
  name: string;
  email: string;
  isUser: boolean;
};

export type ClientFieldDisplay = {
  label: string;
  value?: string;
};

export type ClientEvent = {
  id: number;
  eventType: string;
  eventData: Record<string, unknown>;
  createdAt: string;
  manager: Manager;
};

export type ClientDetails = {
  client: {
    id: number;
    publicId?: string;
    name: string;
    email?: string;
    phone?: string;
    fields: Record<string, string>;
  };
  managers: Manager[];
  events: ClientEvent[];
  documents: ClientEvent[];
  availableFields: string[];
  importantFields: ClientFieldDisplay[];
  otherFields: ClientFieldDisplay[];
  totalEvents: number;
  todoServiceUrl: string;
  filesServiceUrl: string;
};

export type ManagerWithClients = {
  manager: Manager;
  clients: ClientListItem[];
};

export type ManagersData = {
  managers: ManagerWithClients[];
};

export type ManagerModalData = {
  manager: Manager;
  clients: ClientListItem[];
};

export type ImportantFieldSettingsData = {
  fieldsText: string;
};

export type AuthUserSearchItem = {
  sub: string;
  name: string;
  email: string;
};
