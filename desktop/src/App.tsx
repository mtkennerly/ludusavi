import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// Mirrors resource::sync_state::GameSyncEntry.
interface GameSyncEntry {
  last_push: string;
  device: string;
  mapping_path: string;
}

function App() {
  const [games, setGames] = useState<string[]>([]);
  const [status, setStatus] = useState<Record<string, GameSyncEntry | null>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>("enabled_games")
      .then(setGames)
      .catch((e) => setError(String(e)));
  }, []);

  async function checkStatus(game: string) {
    try {
      const entry = await invoke<GameSyncEntry | null>("sync_status", { game });
      setStatus((prev) => ({ ...prev, [game]: entry }));
    } catch (e) {
      setError(String(e));
    }
  }

  async function push(game: string) {
    setBusy(game);
    setError(null);
    try {
      const changeCount = await invoke<number>("sync_push", { game, preview: false });
      await checkStatus(game);
      console.log(`pushed ${game}: ${changeCount} change(s)`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function pull(game: string) {
    setBusy(game);
    setError(null);
    try {
      const changeCount = await invoke<number>("sync_pull", { game, preview: false });
      await checkStatus(game);
      console.log(`pulled ${game}: ${changeCount} change(s)`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }

  return (
    <main className="container">
      <h1>Ludusavi Sync</h1>
      <p>
        Scaffold: this list comes from <code>config.yaml</code>'s{" "}
        <code>sync.enabled_games</code> via the <code>enabled_games</code> Tauri
        command, which calls straight into <code>api.rs::Ludusavi</code> - the
        same struct the CLI's <code>ludusavi sync</code> subcommand uses.
      </p>

      {error && <p style={{ color: "red" }}>{error}</p>}

      {games.length === 0 && !error && (
        <p>
          No games enabled for sync yet. Enable some in <code>config.yaml</code>{" "}
          (<code>sync.enabled_games</code>), or run{" "}
          <code>ludusavi manifest update</code> first if this list failed to load.
        </p>
      )}

      <ul className="game-list">
        {games.map((game) => {
          const entry = status[game];
          return (
            <li key={game}>
              <span className="game-name">{game}</span>
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
            </li>
          );
        })}
      </ul>
    </main>
  );
}

export default App;
