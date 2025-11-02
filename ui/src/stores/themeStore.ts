import { create } from "zustand";
import { persist } from "zustand/middleware";

type Theme = "light" | "dark" | "system";

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      theme: "system",
      setTheme: (theme) => set({ theme }),
      toggleTheme: () =>
        set((state) => {
          const nextTheme: Record<Theme, Theme> = {
            light: "dark",
            dark: "system",
            system: "light",
          };
          return { theme: nextTheme[state.theme] };
        }),
    }),
    {
      name: "theme-storage", // a unique name for the localStorage key
    }
  )
);
