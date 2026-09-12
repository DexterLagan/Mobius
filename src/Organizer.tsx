import { useEffect, useState } from "react";
import type { OrganizerItem } from "./types";
import { formatBytes, organizeFiles, scanDownloads } from "./lib";

interface Row extends OrganizerItem {
  destination: string;
  remember: boolean;
  selected: boolean;
  editing: boolean;
}

interface Props {
  onClose: () => void;
  onDone: () => void;
  onToast: (message: string) => void;
}

export default function Organizer({ onClose, onDone, onToast }: Props) {
  const [rows, setRows] = useState<Row[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    scanDownloads()
      .then((items) =>
        setRows(
          items.map((item) => ({
            ...item,
            destination: item.suggested,
            remember: false,
            selected: true,
            editing: false,
          }))
        )
      )
      .catch((error) => onToast(String(error)))
      .finally(() => setLoading(false));
  }, [onToast]);

  const updateRow = (path: string, patch: Partial<Row>) =>
    setRows((prev) => prev.map((row) => (row.path === path ? { ...row, ...patch } : row)));

  const commitEdit = (row: Row, value: string, applyAll: boolean) => {
    if (!value) {
      onToast("Destination cannot be empty");
      return;
    }
    setRows((prev) =>
      prev.map((entry) => {
        if (entry.path === row.path) {
          return { ...entry, destination: value, remember: applyAll, editing: false };
        }
        if (applyAll && entry.ext === row.ext) {
          return { ...entry, destination: value, remember: true };
        }
        return entry;
      })
    );
  };

  const allSelected = rows.length > 0 && rows.every((row) => row.selected);
  const selectedCount = rows.filter((row) => row.selected).length;

  const toggleAll = () => {
    const next = !allSelected;
    setRows((prev) => prev.map((row) => ({ ...row, selected: next })));
  };

  const organize = async () => {
    const selected = rows.filter((row) => row.selected);
    if (selected.length === 0) {
      onToast("Select at least one file");
      return;
    }
    setBusy(true);
    try {
      await organizeFiles(
        selected.map((row) => ({
          path: row.path,
          destination: row.destination,
          remember: row.remember,
        }))
      );
      onToast(`Organized ${selected.length} file${selected.length === 1 ? "" : "s"}`);
      onDone();
      onClose();
    } catch (error) {
      onToast(String(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div className="modal" onClick={(event) => event.stopPropagation()}>
        <div className="modal-head">
          <div>
            <h2>Organize existing files</h2>
            <p className="muted">
              Files currently in Downloads and the folder each would move to, using the
              configured rules. Edit a destination to override it.
            </p>
          </div>
          <button type="button" className="btn ghost" onClick={onClose}>
            Close
          </button>
        </div>

        {loading ? (
          <p className="muted">Scanning…</p>
        ) : rows.length === 0 ? (
          <p className="muted">No files in the watched folder.</p>
        ) : (
          <div className="modal-body">
            <table className="table">
              <thead>
                <tr>
                  <th className="check-col">
                    <input type="checkbox" checked={allSelected} onChange={toggleAll} />
                  </th>
                  <th>File</th>
                  <th>Destination</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.path}>
                    <td className="check-col">
                      <input
                        type="checkbox"
                        checked={row.selected}
                        onChange={(event) =>
                          updateRow(row.path, { selected: event.target.checked })
                        }
                      />
                    </td>
                    <td>
                      <div className="file-name">{row.fileName}</div>
                      <div className="file-meta">{formatBytes(row.size)}</div>
                    </td>
                    <td>
                      {row.editing ? (
                        <DestinationEditor
                          row={row}
                          onCommit={(value, applyAll) => commitEdit(row, value, applyAll)}
                          onCancel={() => updateRow(row.path, { editing: false })}
                        />
                      ) : (
                        <span className="dest-cell">
                          {row.destination}
                          {row.remember && <span className="rule-tag">rule</span>}
                        </span>
                      )}
                    </td>
                    <td className="right">
                      {!row.editing && (
                        <button
                          type="button"
                          className="btn ghost"
                          onClick={() => updateRow(row.path, { editing: true })}
                        >
                          Edit
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div className="modal-footer">
          <span className="muted">
            {selectedCount} of {rows.length} selected
          </span>
          <div className="card-actions">
            <button type="button" className="btn ghost" onClick={onClose}>
              Cancel
            </button>
            <button
              type="button"
              className="btn primary"
              disabled={busy || selectedCount === 0}
              onClick={organize}
            >
              {busy ? "Organizing…" : "Organize selected"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function DestinationEditor({
  row,
  onCommit,
  onCancel,
}: {
  row: Row;
  onCommit: (value: string, applyAll: boolean) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(row.destination);
  const [applyAll, setApplyAll] = useState(row.remember);

  return (
    <div className="editor">
      <input
        className="text-input wide"
        value={value}
        autoFocus
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") onCommit(value.trim(), applyAll);
          if (event.key === "Escape") onCancel();
        }}
      />
      {row.ext && (
        <label className="check small">
          <input
            type="checkbox"
            checked={applyAll}
            onChange={(event) => setApplyAll(event.target.checked)}
          />
          Apply to all .{row.ext} files
        </label>
      )}
      <div className="editor-actions">
        <button type="button" className="btn ghost" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className="btn primary"
          onClick={() => onCommit(value.trim(), applyAll)}
        >
          Done
        </button>
      </div>
    </div>
  );
}