import { useState } from "react";
import type { Config } from "./types";
import { setConfig } from "./lib";

interface Props {
  config: Config;
  onConfig: (config: Config) => void;
  onToast: (message: string) => void;
}

const CATEGORY_DEFAULTS: Record<string, string> = {
  Documents: "Documents",
  Images: "Images",
  Audio: "Audio",
  Video: "Video",
  Archives: "Archives",
  Code: "Code",
  Apps: "Apps",
  Installers: "Installers",
  Other: "Other",
};

export default function Rules({ config, onConfig, onToast }: Props) {
  const [newExt, setNewExt] = useState("");
  const [newDest, setNewDest] = useState("");

  const persist = async (next: Config) => {
    try {
      await setConfig(next);
      onConfig(next);
    } catch (error) {
      onToast(String(error));
    }
  };

  const addRule = async () => {
    const ext = newExt.trim().replace(/^\./, "").toLowerCase();
    const dest = newDest.trim();
    if (!ext || !dest) {
      onToast("Enter both an extension and a destination");
      return;
    }
    await persist({ ...config, rules: { ...config.rules, [ext]: dest } });
    setNewExt("");
    setNewDest("");
  };

  const removeRule = (ext: string) => {
    const rules = { ...config.rules };
    delete rules[ext];
    persist({ ...config, rules });
  };

  const setCategory = (category: string, value: string) => {
    const categoryDestinations = { ...config.categoryDestinations };
    if (value.trim()) {
      categoryDestinations[category] = value.trim();
    } else {
      delete categoryDestinations[category];
    }
    persist({ ...config, categoryDestinations });
  };

  const ruleEntries = Object.entries(config.rules);

  return (
    <div className="panel">
      <h2>Rules</h2>
      <p className="muted">
        Learned from “Remember for this file type”. A rule always wins over category and
        built-in defaults.
      </p>
      <table className="table">
        <thead>
          <tr>
            <th>Extension</th>
            <th>Destination</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {ruleEntries.length === 0 && (
            <tr>
              <td colSpan={3} className="muted">
                No rules yet.
              </td>
            </tr>
          )}
          {ruleEntries.map(([ext, dest]) => (
            <tr key={ext}>
              <td>.{ext}</td>
              <td>{dest}</td>
              <td className="right">
                <button type="button" className="btn ghost" onClick={() => removeRule(ext)}>
                  Delete
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <div className="option-row add-rule">
        <input
          className="text-input"
          placeholder="ext (e.g. pdf)"
          value={newExt}
          onChange={(event) => setNewExt(event.target.value)}
        />
        <input
          className="text-input wide"
          placeholder="Destination (e.g. Documents/PDFs)"
          value={newDest}
          onChange={(event) => setNewDest(event.target.value)}
        />
        <button type="button" className="btn primary" onClick={addRule}>
          Add rule
        </button>
      </div>

      <h2>Category destinations</h2>
      <p className="muted">
        A per-category override redirects every extension in that category at once.
        Leave blank to use the built-in defaults.
      </p>
      <table className="table">
        <thead>
          <tr>
            <th>Category</th>
            <th>Destination</th>
          </tr>
        </thead>
        <tbody>
          {Object.keys(CATEGORY_DEFAULTS).map((category) => (
            <tr key={category}>
              <td>{category}</td>
              <td>
                <input
                  className="text-input wide"
                  placeholder={CATEGORY_DEFAULTS[category]}
                  value={config.categoryDestinations[category] ?? ""}
                  onChange={(event) => setCategory(category, event.target.value)}
                />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}