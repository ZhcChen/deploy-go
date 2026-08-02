import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { AppProviders } from "./app/AppProviders";
import { AppRoutes } from "./routes/AppRoutes";
import "./styles/index.css";

const root = document.getElementById("root");
const router = createBrowserRouter([{ path: "*", element: <AppRoutes /> }]);

if (!root) throw new Error("缺少 #root 挂载节点");

createRoot(root).render(
  <StrictMode>
    <AppProviders><RouterProvider router={router} /></AppProviders>
  </StrictMode>,
);
