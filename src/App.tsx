import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Config, HistoryEntry, QueueItem, Status } from "./types";
import * as api from "./lib";
import QueueCard from "./QueueCard";
import Settings from "./Settings";
import Rules from "./Rules";
import HistoryPanel from "./HistoryPanel";
import Organizer from "./Organizer";

type Tab = "downloads" | "rules" | "history" | "settings";

export default function App() {
  const [tab, setTab] = useState<Tab>("downloads");
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [config, setConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [lastAction, setLastAction] = useState<string | null>(null);
  const [showOrganizer, setShowOrganizer] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [nextQueue, nextHistory, nextConfig, nextStatus] = await Promise.all([
        api.listQueue(),
        api.listHistory(),
        api.getConfig(),
        api.getStatus(),
      ]);
      setQueue(nextQueue);
      setHistory(nextHistory);
      setConfig(nextConfig);
      setStatus(nextStatus);
    } catch (error) {
      setToast(String(error));
    }
  }, []);

  useEffect(() => {
    refresh();

    const unlisten = [
      listen<QueueItem[]>("queue:updated", (event) => setQueue(event.payload)),
      listen<HistoryEntry[]>("history:updated", (event) => setHistory(event.payload)),
      listen<string>("move:failed", (event) => setToast(event.payload)),
      listen<QueueItem>("file:detected", () => refresh()),
      listen("config:updated", () => refresh()),
      listen("paused:updated", () => refresh()),
      listen<string>("navigate", (event) => {
        const target = event.payload as Tab;
        if (["downloads", "rules", "history", "settings"].includes(target)) {
          setTab(target);
        }
      }),
    ];
    const timer = window.setInterval(refresh, 5000);

    return () => {
      unlisten.forEach((promise) => promise.then((fn) => fn()));
      window.clearInterval(timer);
    };
  }, [refresh]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 4000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const handleChanged = useCallback(
    (message: string) => {
      setLastAction(message);
      setToast(message);
      refresh();
    },
    [refresh]
  );

  const handleDismiss = useCallback(
    async (path: string) => {
      await api.dismiss(path);
      refresh();
    },
    [refresh]
  );

  const undoLast = async () => {
    const latest = history.find((entry) => !entry.undone);
    if (!latest) {
      setToast("Nothing to undo");
      return;
    }
    try {
      await api.undo(latest.id);
      setToast("Move undone");
      refresh();
    } catch (error) {
      setToast(String(error));
    }
  };

  if (!config) {
    return <div className="loading">Loading Mobius…</div>;
  }

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="logo">◐</span> Mobius
        </div>
        <nav className="tabs">
          {(["downloads", "rules", "history", "settings"] as Tab[]).map((item) => (
            <button
              key={item}
              type="button"
              className={`tab ${tab === item ? "active" : ""}`}
              onClick={() => setTab(item)}
            >
              {item === "downloads" ? `Downloads${queue.length ? ` (${queue.length})` : ""}` : item[0].toUpperCase() + item.slice(1)}
            </button>
          ))}
        </nav>
      </header>

      <div className="statusbar">
        <span title={status?.watchDir}>Watching {status?.watchDir}</span>
        <span className="dot" /> {queue.length} pending
        {status?.paused && <span className="badge">Paused</span>}
        {status?.lastScan && (
          <span className="muted">· last check {new Date(status.lastScan).toLocaleTimeString()}</span>
        )}
      </div>

      <main className="content">
        {tab === "downloads" && (
          <div className="panel">
            <div className="panel-head">
              <h2>New downloads</h2>
              <button
                type="button"
                className="btn ghost"
                onClick={() => setShowOrganizer(true)}
              >
                Organize existing files…
              </button>
            </div>
            {queue.length === 0 ? (
              <div className="empty">
                <p className="muted">
                  Nothing waiting. New files that finish downloading will show up here.
                </p>
              </div>
            ) : (
              queue.map((item) => (
                <QueueCard
                  key={item.id}
                  item={item}
                  count={queue.length}
                  onChanged={handleChanged}
                  onDismiss={handleDismiss}
                  onToast={setToast}
                />
              ))
            )}
          </div>
        )}

        {tab === "rules" && (
          <Rules config={config} onConfig={setConfig} onToast={setToast} />
        )}
        {tab === "history" && <HistoryPanel history={history} onToast={setToast} />}
        {tab === "settings" && (
          <Settings config={config} onConfig={setConfig} onToast={setToast} />
        )}
      </main>

      {lastAction && history.length > 0 && tab === "downloads" && (
        <div className="undo-bar">
          <span>{lastAction}</span>
          <button type="button" className="btn ghost" onClick={undoLast}>
            Undo
          </button>
        </div>
      )}

      {showOrganizer && (
        <Organizer
          onClose={() => setShowOrganizer(false)}
          onDone={refresh}
          onToast={setToast}
        />
      )}

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}