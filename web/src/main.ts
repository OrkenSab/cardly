import { mount } from "svelte";
import "./styles/anki-tokens.css";
import App from "./App.svelte";
import { initPlugins } from "./lib/plugins";
import { store } from "./lib/store.svelte";

// Register Service Worker for offline support and PWA
if ("serviceWorker" in navigator && window.location.protocol.startsWith("http")) {
  window.addEventListener("load", () => {
    navigator.serviceWorker
      .register("/sw.js")
      .then((reg) => {
        // Check for updates periodically
        reg.addEventListener("updatefound", () => {
          const newWorker = reg.installing;
          newWorker?.addEventListener("statechange", () => {
            if (newWorker.state === "installed" && navigator.serviceWorker.controller) {
              store.toast("Cardly updated in background. Refresh to use latest version.", "info", 4000);
            }
          });
        });
      })
      .catch((err) => {
        console.warn("ServiceWorker registration:", err);
      });
  });
}

// PWA install prompt capture
window.addEventListener("beforeinstallprompt", (e) => {
  e.preventDefault();
  store.setInstallPrompt(e);
});

// Network online / offline events
window.addEventListener("online", () => store.setOnline(true));
window.addEventListener("offline", () => store.setOnline(false));

// Global unhandled error boundary
window.addEventListener("error", (ev) => {
  console.error("Cardly Uncaught Error:", ev.error || ev.message);
});

window.addEventListener("unhandledrejection", (ev) => {
  console.error("Cardly Unhandled Rejection:", ev.reason);
});

initPlugins();

mount(App, {
  target: document.getElementById("app")!,
});

