import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { api, type Account } from "../lib/api";

const STORAGE_KEY = "activeAccountId";

interface AccountContextValue {
  accounts: Account[];
  activeAccountId: string | null;
  setActiveAccount: (id: string) => void;
  refresh: () => Promise<void>;
}

const AccountContext = createContext<AccountContextValue | null>(null);

function readStoredId(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function persistId(id: string | null): void {
  try {
    if (id === null) localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, id);
  } catch {
    // Ignore storage failures (private mode, quota) — state still works in-memory.
  }
}

export function AccountProvider({ children }: { children: ReactNode }) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [activeAccountId, setActiveAccountId] = useState<string | null>(() =>
    readStoredId(),
  );
  const mounted = useRef(true);

  const reconcile = useCallback((list: Account[]) => {
    const safe = Array.isArray(list) ? list : [];
    setAccounts(safe);
    setActiveAccountId((current) => {
      // Keep the current choice if it still exists; otherwise fall back to the
      // first account (or null when there are none).
      if (current && safe.some((a) => a.id === current)) return current;
      const next = safe[0]?.id ?? null;
      persistId(next);
      return next;
    });
  }, []);

  const refresh = useCallback(async () => {
    try {
      const list = await api.listAccounts();
      if (mounted.current) reconcile(list);
    } catch {
      // Degrade gracefully — never throw out of the provider.
      if (mounted.current) reconcile([]);
    }
  }, [reconcile]);

  const setActiveAccount = useCallback((id: string) => {
    setActiveAccountId(id);
    persistId(id);
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    return () => {
      mounted.current = false;
    };
  }, [refresh]);

  const value = useMemo<AccountContextValue>(
    () => ({ accounts, activeAccountId, setActiveAccount, refresh }),
    [accounts, activeAccountId, setActiveAccount, refresh],
  );

  return (
    <AccountContext.Provider value={value}>{children}</AccountContext.Provider>
  );
}

export function useAccounts(): AccountContextValue {
  const ctx = useContext(AccountContext);
  if (!ctx) {
    throw new Error("useAccounts must be used within an AccountProvider");
  }
  return ctx;
}
