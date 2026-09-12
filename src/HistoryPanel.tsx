import type { HistoryEntry } from "./types";
import { undo } from "./lib";

interface Props {
  history: HistoryEntry[];
  onToast: (message: string) => void;
}

export default function HistoryPanel({ history, onToast }: Props) {
  const handleUndo = async (id: string) => {
    try {
      await undo(id);
      onToast("Move undone");
    } catch (error) {
      onToast(String(error));
    }
  };

  return (
    <div className="panel">
      <h2>History</h2>
      {history.length === 0 ? (
        <p className="muted">No moves yet.</p>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>File</th>
              <th>Moved to</th>
              <th>When</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {history.map((entry) => (
              <tr key={entry.id} className={entry.undone ? "undone" : ""}>
                <td>{entry.source.split(/[\\/]/).pop()}</td>
                <td className="ellipsis" title={entry.destination}>
                  {entry.destination}
                </td>
                <td>{new Date(entry.timestamp).toLocaleString()}</td>
                <td className="right">
                  {entry.undone ? (
                    <span className="muted">Undone</span>
                  ) : (
                    <button
                      type="button"
                      className="btn ghost"
                      onClick={() => handleUndo(entry.id)}
                    >
                      Undo
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}