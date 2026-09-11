# Privacy

Shadowmask is **self-hosted software**, not a service. There is no Shadowmask account, no Shadowmask
server, and no company behind it collecting anything. You run the server; your data stays on the
machine you run it on.

This document describes what the software stores, what it sends, and to whom. It applies to the
server and to every client (web, desktop, Android, iOS, Roku, and the basic HTML client).

## The short version

- **The maintainers of this project receive no data from you. None.** There is no backend to receive
  it, and nothing in the code contacts us.
- **No telemetry, no analytics, no crash reporting, no update check.** The clients talk to the server
  you point them at, and to nothing else.
- **Anything that leaves your server leaves because an operator turned it on** — metadata lookups,
  subtitle downloads, remote content fetching — and each is off until configured.

## What the server stores

All of it lives in your database and cache directories, on your machine:

- **Accounts** — username, a hashed password (never the password itself), role, profile settings, and
  which libraries each account may see.
- **Viewing data** — watch history, resume positions, watchlist, favorites, and playback sessions
  while they are active.
- **Devices and tokens** — the devices provisioned by link code, and hashes of their API tokens. The
  server stores only a SHA-256 hash of an API token, never the token.
- **Your media and what was learned about it** — the library contents, plus metadata, artwork,
  subtitles and trickplay thumbnails derived from them.
- **Operational logs** — background job records and log lines. These name user ids and the actions
  taken, because that is what makes a server diagnosable.

**The server does not record client IP addresses.** If you put a reverse proxy in front of it, that
proxy may keep its own access logs; those are yours to configure.

## What a client stores on your device

The address of your server, the credentials or device token it was issued, and per-device
preferences (theme, player defaults, language). Signing out clears them.

## What leaves your server, and only if you configure it

Shadowmask ships with every outbound integration **off**. When an operator enables one, these are
the third parties involved, and each has its own privacy policy:

| Integration               | Who it contacts                               | What is sent                                                                                  |
|---------------------------|-----------------------------------------------|-----------------------------------------------------------------------------------------------|
| Metadata (TMDB)           | `api.themoviedb.org`, `image.tmdb.org`        | Title and year search terms derived from your file names; artwork is downloaded by the server |
| Metadata (OMDb)           | `www.omdbapi.com`                             | The same kind of search terms                                                                 |
| Subtitles (OpenSubtitles) | `api.opensubtitles.com`                       | Search terms identifying the title you want subtitles for                                     |
| Remote content fetch      | Whatever site an admin gives it, via `yt-dlp` | The request an admin explicitly asked for                                                     |

Two things worth knowing about how this is arranged:

- **Artwork is fetched by the server, not by your clients.** The server downloads images once and
  serves them from your own machine, so browsing your library does not put your devices in contact
  with a metadata provider.
- **The AI features are local.** Transcription, translation and upscaling run models on your own
  hardware. No audio, video or text is sent anywhere for them.

The only other way you reach a third party is by choosing to: a title page offers links to IMDb and
TMDB, and following one is an ordinary visit to their site.

## If you run a server for other people

When you host Shadowmask for family, friends or anyone else, **you hold their data, not us**. Their
accounts, viewing history and devices sit in your database, and the choices about retention, access
and deletion are yours. The server gives you the tools — deleting a user removes their data and their
per-user directory, and history can be cleared per title or entirely.

## Questions

This policy describes the software as it is built. If you find something in the code that contradicts
it, that is a bug worth reporting — open an issue, or see [SECURITY.md](SECURITY.md) if it is a
vulnerability.
