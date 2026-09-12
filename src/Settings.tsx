import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { Config } from "./types";
import { setConfig } from "./lib";

interface Props {
  config: Config;
  onConfig: (config: Config) => void;
  onToast: (message: string) => void;
}

export default function Settings({ config, onConfig, onToast }: Props) {
  const [draft, setDraft] = useState<Config>(config);
  const [saving, setSaving] = useState(false);

  const update = (patch: Partial<Config>) => setDraft({ ...draft, ...patch });

  const browse = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose the watched folder",
    });
    if (typeof selected === "string") update({ watchDir: selected });
  };

  const save = async () => {
    setSaving(true);
    try {
      await setConfig(draft);
      onConfig(draft);
      onToast("Settings saved");
    } catch (error) {
      onToast(String(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="panel">
      <h2>Settings</h2>

      <div className="field">
        <label>Watched folder</label>
        <div className="option-row">
          <input
            className="text-input wide"
            value={draft.watchDir}
            onChange={(event) => update({ watchDir: event.target.value })}
          />
          <button type="button" className="btn ghost" onClick={browse}>
            Browse…
          </button>
        </div>
      </div>

      <div className="grid">
        <div className="field">
          <label>Scan interval (ms)</label>
          <input
            type="number"
            className="text-input"
            min={250}
            value={draft.scanIntervalMs}
            onChange={(event) => update({ scanIntervalMs: Number(event.target.value) })}
          />
        </div>
        <div className="field">
          <label>Batch window (ms)</label>
          <input
            type="number"
            className="text-input"
            min={0}
            value={draft.batchWindowMs}
            onChange={(event) => update({ batchWindowMs: Number(event.target.value) })}
          />
        </div>
        <div className="field">
          <label>Duplicate policy</label>
          <select
            className="text-input"
            value={draft.duplicatePolicy}
            onChange={(event) => update({ duplicatePolicy: event.target.value })}
          >
            <option value="version">Version (Previous Versions folder)</option>
            <option value="rename">Rename (name (1).ext)</option>
            <option value="replace">Replace</option>
            <option value="skip">Skip</option>
          </select>
        </div>
        <div className="field">
          <label>Undo toast (ms)</label>
          <input
            type="number"
            className="text-input"
            min={0}
            value={draft.undoToastMs}
            onChange={(event) => update({ undoToastMs: Number(event.target.value) })}
          />
        </div>
      </div>

      <label className="check">
        <input
          type="checkbox"
          checked={draft.flattenToCategories}
          onChange={(event) => update({ flattenToCategories: event.target.checked })}
        />
        Flatten to category folders
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={draft.allowExternalDestinations}
          onChange={(event) => update({ allowExternalDestinations: event.target.checked })}
        />
        Allow destinations outside the watched folder
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={draft.versionTimestampPrefix}
          onChange={(event) => update({ versionTimestampPrefix: event.target.checked })}
        />
        Timestamp previous versions
      </label>

      <div className="card-actions">
        <button type="button" className="btn primary" disabled={saving} onClick={save}>
          {saving ? "Saving…" : "Save"}
        </button>
      </div>
    </div>
  );
}