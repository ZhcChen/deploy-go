import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { AppProviders } from "./app/AppProviders";
import { AppRoutes } from "./routes/AppRoutes";
import "./styles/index.css";

const root = document.getElementById("root");

if (!root) throw new Error("缺少 #root 挂载节点");

createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      <AppProviders>
        <AppRoutes />
      </AppProviders>
    </BrowserRouter>
  </StrictMode>,
);
