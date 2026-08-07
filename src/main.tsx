import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App.tsx";
import "./index.css";

// Hide splash screen once React mounts
const splash = document.getElementById("splash");
if (splash) {
  // Show splash for at least 1.5s for visual impact, then fade out
  setTimeout(() => {
    splash.classList.add("hidden");
    setTimeout(() => splash.remove(), 500);
  }, 1500);
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
