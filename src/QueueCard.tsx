import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { QueueItem } from "./types";
import { confirmMove, formatBytes } from "./lib";

interface Props {
  item: QueueItem;
  count: number;
  onChanged: (message: string) => void;
  onDismiss: (path: string) => void;
  onToast: (message: string) => void;
}

export default function QueueCard({ item, count, onChanged, onDismiss, onToast }: Props) {
  const [choice, setChoice] = useState<"suggested" | "custom">("suggested");
  const [custom, setCustom] = useState("");
  const [remember, setRemember] = useState(false);
  const [applyAll, setApplyAll] = useState(false);
  const [busy, setBusy] = useState(false);

  const browse = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose destination folder",
    });
    if (typeof selected === "string") {
      setCustom(selected);
      setChoice("custom");
    }
  };

  const move = async () => {
    if (choice === "custom" && !custom.trim()) {
      onToast("Enter a folder name or pick one");
      return;
    }
    setBusy(true);
    try {
      await confirmMove({
        path: item.path,
        customDestination: choice === "custom" ? custom.trim() : null,
        remember,
        applyAll,
      });
      onChanged(`Moved ${item.fileName}`);
    } catch (error) {
      onToast(String(error));
    } finally {
      setBusy(false);
    }
  };

  const preview = remember
    ? `Next .${item.ext || "?"} → ${choice === "custom" ? custom.trim() || "…" : item.suggested}`
    : null;

  return (
    <div className="card">
      <div className="card-head">
        <div>
          <div className="file-name">{item.fileName}</div>
          <div className="file-meta">
            {item.ext ? `.${item.ext}` : "no extension"} · {formatBytes(item.size)} ·{" "}
            {new Date(item.detectedAt).toLocaleTimeString()}
          </div>
        </div>
        <span className="badge">{item.suggested}</span>
      </div>

      <label className={`option ${choice === "suggested" ? "selected" : ""}`}>
        <input
          type="radio"
          name={`choice-${item.id}`}
          checked={choice === "suggested"}
          onChange={() => setChoice("suggested")}
        />
        <span>
          Move to <strong>{item.suggested}</strong>
        </span>
      </label>

      <label className={`option ${choice === "custom" ? "selected" : ""}`}>
        <input
          type="radio"
          name={`choice-${item.id}`}
          checked={choice === "custom"}
          onChange={() => setChoice("custom")}
        />
        <span className="option-row">
          Move to
          <input
            className="text-input"
            placeholder="Folder name or path"
            value={custom}
            onFocus={() => setChoice("custom")}
            onChange={(event) => setCustom(event.target.value)}
          />
          <button type="button" className="btn ghost" onClick={browse}>
            Browse…
          </button>
        </span>
      </label>

      <div className="checks">
        {count > 1 && (
          <label className="check">
            <input
              type="checkbox"
              checked={applyAll}
              onChange={(event) => setApplyAll(event.target.checked)}
            />
            Apply to all current downloads ({count})
          </label>
        )}
        <label className="check">
          <input
            type="checkbox"
            checked={remember}
            onChange={(event) => setRemember(event.target.checked)}
          />
          Remember for this file type (.{item.ext || "?"})
        </label>
      </div>

      {preview && <div className="rule-preview">{preview}</div>}

      <div className="card-actions">
        <button
          type="button"
          className="btn ghost"
          disabled={busy}
          onClick={() => onDismiss(item.path)}
        >
          Do Nothing
        </button>
        <button type="button" className="btn primary" disabled={busy} onClick={move}>
          {busy ? "Moving…" : "Move"}
        </button>
      </div>
    </div>
  );
}