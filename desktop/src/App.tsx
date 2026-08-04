import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CloudSettings } from "./CloudSettings";
import "./App.css";

// Mirrors resource::sync_state::GameSyncEntry.
interface GameSyncEntry {
  last_push: string;
  device: string;
  mapping_path: string;
}

// Mirrors src-tauri's ScanResult.
interface ScanResult {
  name: string;
  file_count: number;
  registry_count: number;
  change: string;
}

// Mirrors src-tauri's SyncProgressEvent.
interface SyncProgressEvent {
  game: string;
  current: number;
  total: number;
}

// Live byte progress of a push/pull for one game.
interface GameProgress {
  current: number;
  total: number;
}

function formatBytes(bytes: number): string {
  if (!isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  const exp = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, exp);
  return `${value.toFixed(value >= 100 || exp === 0 ? 0 : 1)} ${units[exp]}`;
}

type Page = "sync" | "settings";

function App() {
  const [page, setPage] = useState<Page>("sync");
  const [scanning, setScanning] = useState(false);

  return (
    <main className="container">
      <header className="app-header">
        <h1>Ludusavi Sync</h1>
        {page === "sync" ? (
          <button
            className="icon-button"
            aria-label="Settings"
            title="Settings"
            disabled={scanning}
            onClick={() => setPage("settings")}
          >
            ⚙
          </button>
        ) : (
          <button className="icon-button" aria-label="Back" title="Back" onClick={() => setPage("sync")}>
            ←
          </button>
        )}
      </header>

      {page === "sync" ? (
        <SyncScreen scanning={scanning} onScanningChange={setScanning} />
      ) : (
        <CloudSettings />
      )}
    </main>
  );
}

function SyncScreen({
  scanning,
  onScanningChange,
}: {
  scanning: boolean;
  onScanningChange: (scanning: boolean) => void;
}) {
  const [enabledGames, setEnabledGames] = useState<string[]>([]);
  const [discoveredGames, setDiscoveredGames] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<string[]>([]);
  const [scanResults, setScanResults] = useState<Record<string, ScanResult>>({});
  const [statuses, setStatuses] = useState<Record<string, GameSyncEntry | null>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<Record<string, GameProgress | null>>({});

  // Bumped on every new scan and on cancel, so a stale scan's late resolution
  // (and its partial results) can be ignored instead of clobbering the UI.
  const scanIdRef = useRef(0);

  function refreshEnabled() {
    invoke<string[]>("enabled_games")
      .then(setEnabledGames)
      .catch((e) => setError(String(e)));
  }

  function refreshDiscovered() {
    invoke<string[]>("discovered_games")
      .then(setDiscoveredGames)
      .catch((e) => setError(String(e)));
  }

  // Load both the starred games (always shown) and the persisted scan results,
  // so a restart doesn't force a manual re-scan.
  useEffect(() => {
    refreshEnabled();
    refreshDiscovered();
  }, []);

  // Only search once there's a query - avoids fetching/rendering the whole
  // (potentially 19,000+ game) manifest by default. With no query, the list
  // below is just whatever's already enabled (plus anything found by Scan).
  useEffect(() => {
    if (query.trim() === "") {
      setSearchResults([]);
      return;
    }
    const handle = setTimeout(() => {
      invoke<string[]>("search_games", { query })
        .then(setSearchResults)
        .catch((e) => setError(String(e)));
    }, 150);
    return () => clearTimeout(handle);
  }, [query]);

  async function scan() {
    const id = ++scanIdRef.current;
    onScanningChange(true);
    setError(null);
    try {
      const results = await invoke<ScanResult[]>("scan_games");
      if (id !== scanIdRef.current) return;
      setScanResults(Object.fromEntries(results.map((r) => [r.name, r])));
      refreshDiscovered();
    } catch (e) {
      if (id === scanIdRef.current) setError(String(e));
    } finally {
      if (id === scanIdRef.current) onScanningChange(false);
    }
  }

  async function cancelScan() {
    scanIdRef.current++; // invalidate the in-flight scan so its results are dropped
    onScanningChange(false);
    try {
      await invoke("cancel_scan");
    } catch (e) {
      setError(String(e));
    }
  }

  async function toggleEnabled(game: string, enabled: boolean) {
    try {
      await invoke("set_game_enabled", { game, enabled });
      refreshEnabled();
    } catch (e) {
      setError(String(e));
    }
  }

  async function checkStatus(game: string) {
    try {
      const entry = await invoke<GameSyncEntry | null>("sync_status", { game });
      setStatuses((prev) => ({ ...prev, [game]: entry }));
    } catch (e) {
      setError(String(e));
    }
  }

  async function backup(game: string) {
    setBusy(game);
    setError(null);
    try {
      await invoke<number>("backup_game", { game });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }

  // Run a push or pull, streaming the backend's "sync-progress" events into the
  // matching game's bar. The event listener is torn down when the invoke settles.
  async function syncTransfer(game: string, op: "sync_push" | "sync_pull") {
    setBusy(game);
    setError(null);
    setProgress((prev) => ({ ...prev, [game]: { current: 0, total: 0 } }));
    const unlisten = await listen<SyncProgressEvent>("sync-progress", (e) => {
      if (e.payload.game !== game) return;
      setProgress((prev) => ({ ...prev, [game]: { current: e.payload.current, total: e.payload.total } }));
    });
    try {
      await invoke<number>(op, { game, preview: false });
      await checkStatus(game);
    } catch (e) {
      setError(String(e));
    } finally {
      unlisten();
      setBusy(null);
      setProgress((prev) => ({ ...prev, [game]: null }));
    }
  }

  async function push(game: string) {
    await syncTransfer(game, "sync_push");
  }

  async function pull(game: string) {
    await syncTransfer(game, "sync_pull");
  }

  // One unified, deduplicated list: everything enabled (starred), plus whatever
  // the search or a Scan turned up that isn't already in that set. Starred games
  // always sort first, so starring/unstarring is what controls both "is this a
  // push/pull target" and "is this near the top".
  const enabledSet = new Set(enabledGames);
  const names = new Set<string>([
    ...enabledGames,
    ...discoveredGames,
    ...searchResults,
    ...Object.keys(scanResults),
  ]);
  const rows = [...names].sort((a, b) => {
    const aEnabled = enabledSet.has(a);
    const bEnabled = enabledSet.has(b);
    if (aEnabled !== bEnabled) return aEnabled ? -1 : 1;
    return a.localeCompare(b);
  });

  return (
    <>
      {error && <p className="error-text">{error}</p>}

      <div className="search-row">
        <input
          className="search-field"
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          placeholder="Search for a game to add..."
        />
        <button disabled={scanning} onClick={scan} title="Scan your configured roots for installed games">
          {scanning ? "Scanning..." : "Scan"}
        </button>
      </div>

      {scanning && (
        <div className="scanning-indicator">
          <div className="spinner" />
          <span>Scanning for games…</span>
          <button onClick={cancelScan} title="Stop the scan">
            Cancel
          </button>
        </div>
      )}

      {rows.length === 0 && (
        <p>
          {query.trim() === ""
            ? "No games enabled yet. Search above, or hit Scan to find installed games."
            : "No matches."}
        </p>
      )}

      <ul className="game-list">
        {rows.map((game) => {
          const enabled = enabledSet.has(game);
          const entry = statuses[game];
          const scanned = scanResults[game];
          const gameProgress = busy === game ? progress[game] : null;
          const pct =
            gameProgress && gameProgress.total > 0
              ? Math.min(100, Math.round((gameProgress.current / gameProgress.total) * 100))
              : null;
          return (
            <li key={game}>
              <div className="game-row">
                <button
                  className="star-button"
                  aria-label={enabled ? "Remove from sync" : "Add to sync"}
                  title={enabled ? "Remove from sync" : "Add to sync"}
                  onClick={() => toggleEnabled(game, !enabled)}
                >
                  {enabled ? "★" : "☆"}
                </button>
                <span className="game-name">{game}</span>
                {enabled ? (
                  <>
                    <button disabled={busy === game} onClick={() => backup(game)}>
                      Backup
                    </button>
                    <button disabled={busy === game} onClick={() => push(game)}>
                      Push
                    </button>
                    <button disabled={busy === game} onClick={() => pull(game)}>
                      Pull
                    </button>
                    <button disabled={busy === game} onClick={() => checkStatus(game)}>
                      Status
                    </button>
                    {entry !== undefined && (
                      <span className="game-status">
                        {entry
                          ? `last pushed ${entry.last_push} from ${entry.device}`
                          : "no cloud sync record"}
                      </span>
                    )}
                  </>
                ) : (
                  scanned && (
                    <span className="game-status">
                      found: {scanned.file_count} file(s), {scanned.change}
                    </span>
                  )
                )}
              </div>
              {gameProgress && (
                <div className="sync-progress">
                  <div
                    className={`sync-progress-fill${pct === null ? " sync-progress-indeterminate" : ""}`}
                    style={pct === null ? undefined : { width: `${pct}%` }}
                  />
                  <span className="sync-progress-label">
                    {pct === null
                      ? "syncing…"
                      : `${pct}% · ${formatBytes(gameProgress.current)} / ${formatBytes(gameProgress.total)}`}
                  </span>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </>
  );
}

export default App;
