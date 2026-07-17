# shadowmask / assets

Shared image assets used by the client subprojects.

The files here are the single source of truth. Make any change here first, then
distribute it to the clients with `./refresh_assets.py`.

- `shadowmask.logo.svg` - the brand mark; source for every derived client asset.

## Usage

```
./refresh_assets.py            # distribute all assets to all clients
./refresh_assets.py -p clients/basic
./refresh_assets.py --verify   # check that distributed copies match the source (CI drift)
./refresh_assets.py -v         # debug logging
```

## Targets

| Project | Source | Target |
|---|---|---|
| `clients/basic` | `shadowmask.logo.svg` | `favicon.svg` |

As the Flutter, Android, iOS, and Roku clients land, add their launcher and icon
targets here. Rasterized PNG sizes (needed by the native launchers) are not generated
yet; they will be pre-rendered into `assets/icons` and `assets/launchers` and
distributed the same way, mirroring the layout in the `stasis` repo.
