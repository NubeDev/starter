import {
  createContext,
  useContext,
  useEffect,
  useReducer,
  type ReactNode,
} from "react";
import type {
  AppState,
  Client,
  SiteInfo,
  DeviceTemplate,
  Project,
} from "@/types";
import { SEED } from "@/data/seed";
import { migrateState } from "@/store/migrate";

const STORAGE_KEY = "pms-poc-state-v1";

function uid(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 9)}`;
}

type Action =
  | { type: "RESET" }
  | { type: "IMPORT"; state: AppState }
  | { type: "ADD_CLIENT"; client: Omit<Client, "id"> }
  | { type: "UPDATE_CLIENT"; client: Client }
  | { type: "DELETE_CLIENT"; id: string }
  | { type: "ADD_SITE"; site: Omit<SiteInfo, "id"> }
  | { type: "UPDATE_SITE"; site: SiteInfo }
  | { type: "DELETE_SITE"; id: string }
  | { type: "ADD_TEMPLATE"; template: Omit<DeviceTemplate, "id"> }
  | { type: "UPDATE_TEMPLATE"; template: DeviceTemplate }
  | { type: "DELETE_TEMPLATE"; id: string }
  | { type: "UPSERT_PROJECT"; project: Project }
  | { type: "DELETE_PROJECT"; id: string };

function load(): AppState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return migrateState(JSON.parse(raw));
  } catch {
    /* ignore */
  }
  return SEED;
}

function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "RESET":
      return SEED;
    case "IMPORT":
      return migrateState(action.state);

    case "ADD_CLIENT":
      return { ...state, clients: [...state.clients, { ...action.client, id: uid("cli") }] };
    case "UPDATE_CLIENT":
      return { ...state, clients: state.clients.map((c) => (c.id === action.client.id ? action.client : c)) };
    case "DELETE_CLIENT":
      return {
        ...state,
        clients: state.clients.filter((c) => c.id !== action.id),
        sites: state.sites.filter((s) => s.clientId !== action.id),
        projects: state.projects.filter((p) => p.clientId !== action.id),
      };

    case "ADD_SITE":
      return { ...state, sites: [...state.sites, { ...action.site, id: uid("site") }] };
    case "UPDATE_SITE":
      return { ...state, sites: state.sites.map((s) => (s.id === action.site.id ? action.site : s)) };
    case "DELETE_SITE":
      return {
        ...state,
        sites: state.sites.filter((s) => s.id !== action.id),
        projects: state.projects.filter((p) => p.siteId !== action.id),
      };

    case "ADD_TEMPLATE":
      return { ...state, templates: [...state.templates, { ...action.template, id: uid("tpl") }] };
    case "UPDATE_TEMPLATE":
      return { ...state, templates: state.templates.map((t) => (t.id === action.template.id ? action.template : t)) };
    case "DELETE_TEMPLATE":
      return { ...state, templates: state.templates.filter((t) => t.id !== action.id) };

    case "UPSERT_PROJECT": {
      const exists = state.projects.some((p) => p.id === action.project.id);
      return {
        ...state,
        projects: exists
          ? state.projects.map((p) => (p.id === action.project.id ? action.project : p))
          : [...state.projects, action.project],
      };
    }
    case "DELETE_PROJECT":
      return { ...state, projects: state.projects.filter((p) => p.id !== action.id) };

    default:
      return state;
  }
}

interface Ctx {
  state: AppState;
  dispatch: React.Dispatch<Action>;
  newId: (prefix: string) => string;
}

const StoreContext = createContext<Ctx | null>(null);

export function StoreProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, undefined, load);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  }, [state]);

  return (
    <StoreContext.Provider value={{ state, dispatch, newId: uid }}>
      {children}
    </StoreContext.Provider>
  );
}

export function useStore(): Ctx {
  const ctx = useContext(StoreContext);
  if (!ctx) throw new Error("useStore must be used within StoreProvider");
  return ctx;
}
