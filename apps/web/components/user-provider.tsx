"use client";

import { useEffect } from "react";
import { create } from "zustand";
import { User } from "@/lib/types";

interface UserState {
  user: User | null;
  loading: boolean;
  loadUser: () => Promise<void>;
  refreshUser: () => Promise<void>;
  setUser: (user: User | null) => void;
}

export const useUserStore = create<UserState>((set, get) => ({
  user: null,
  loading: true,
  setUser: (user) => set({ user }),
  loadUser: async () => {
    set({ loading: true });
    try {
      const res = await fetch("/api/account", { headers: { accept: "application/json" } });
      if (!res.ok) {
        set({ user: null, loading: false });
        return;
      }
      const data = await res.json();
      set({ user: data, loading: false });
    } catch (err) {
      set({ user: null, loading: false });
    }
  },
  refreshUser: async () => {
    await get().loadUser();
  },
}));

export function UserProvider({ children }: { children: React.ReactNode }) {
  const loadUser = useUserStore((state) => state.loadUser);

  useEffect(() => {
    void loadUser();
  }, [loadUser]);

  return <>{children}</>;
}

export function useUser() {
  const user = useUserStore((state) => state.user);
  const loading = useUserStore((state) => state.loading);
  const refreshUser = useUserStore((state) => state.refreshUser);

  return { user, loading, refreshUser };
}
