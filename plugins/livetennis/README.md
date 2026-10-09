# Live tennis

Search tennis score snapshots from Live Tennis API. Open `/livetennis` in Hamr, then search by player or tournament.
Select a match to copy its score and snapshot time.

Set `LIVETENNIS_API_KEY` in the environment that starts the Hamr daemon, then restart it.
The plugin uses `GET /matches?status=live&limit=200`, which works with a free account.
It requires Python 3.9 or newer and uses its standard library.

Scores are snapshots, refreshed on use at most once every 15 minutes.
All searches, plugin copies and process restarts share the same request history for that credential.
Failed requests also start the waiting period. Redirects and retries are disabled.
The request floor allows at most 96 attempts in 24 hours, below the free account's 100 request allowance.
Other applications using the same credential consume that allowance too.

The plugin fetches one page. Its status row shows when more matches exist, including the total when supplied.
Search filters that stored page locally. It does not request extra pages or paid endpoints.
Set scores preserve player order. Tiebreak points are labeled without guessing the set format.
Withheld scores and missing values are shown explicitly. Failed refreshes retain the previous dated snapshot.

Request history lives under `$XDG_CACHE_HOME/hamr/livetennis`, or `~/.cache/hamr/livetennis` when that variable is unset.
Filenames contain a credential hash. The credential is never saved.
The plugin stops requests if the stored history cannot be read, or if it cannot save the next attempt before requesting data.

Run `./plugins/livetennis/test.sh` from the repository root.
The checks use local HTTP fixtures and temporary cache directories. They require no credential or remote service.

For a sample snapshot, start the daemon with `HAMR_TEST_MODE=1`, then run `cargo run -p hamr-tui -- test livetennis Alex`.
Test mode returns synthetic players and never requests API data or writes request history.
