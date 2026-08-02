import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type PropsWithChildren, useState } from "react";
import { AppErrorBoundary } from "./AppErrorBoundary";
import { AuthProvider, type AuthSnapshot } from "../features/auth/AuthContext";

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
        <AuthProvider initialSnapshot={initialAuth}>{children}</AuthProvider>
      </QueryClientProvider>
    </AppErrorBoundary>
  );
}
