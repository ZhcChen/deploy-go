import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type PropsWithChildren, useState } from "react";
import { AppErrorBoundary } from "./AppErrorBoundary";
import { AuthProvider, type AuthSnapshot } from "../features/auth/AuthContext";
import { ThemeProvider } from "../theme/ThemeContext";

export function AppProviders({ children, initialAuth }: PropsWithChildren<{ initialAuth?: AuthSnapshot }>) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { retry: 1, refetchOnWindowFocus: false },
          mutations: { retry: false },
        },
      }),
  );

  return (
    <AppErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider>
          <AuthProvider initialSnapshot={initialAuth}>{children}</AuthProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </AppErrorBoundary>
  );
}
