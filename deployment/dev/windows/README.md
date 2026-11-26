# deployment / dev / windows

A Windows 11 VM (Vagrant, VirtualBox) that runs the Windows desktop client against the dev stack.

## Requirements

- VirtualBox 7.0 or later and Vagrant.
- The dev stack running on the host: `docker compose up --build` in `deployment/dev`.

## Usage

```
vagrant up                                  # install the newest zip in artifacts/
SHADOWMASK_WIN_ARTIFACTS=build vagrant up   # build the client in the VM, then install it
vagrant provision                           # re-run on an existing VM (same variables)
vagrant destroy -f
```

Open Shadowmask from the Start menu, enter `http://10.0.2.2:<SHADOWMASK_PORT>` and sign in as
`admin` (`SHADOWMASK_ADMIN_PASSWORD`, default `passw0rd`).

| Variable                   | Default    | Purpose                                         |
|----------------------------|------------|-------------------------------------------------|
| `SHADOWMASK_WIN_ARTIFACTS` | `released` | `build` builds the client in the VM first       |
| `SHADOWMASK_WIN_MEMORY`    | `8192`     | guest RAM (MB)                                  |
| `SHADOWMASK_WIN_CPUS`      | `4`        | guest vCPUs                                     |
| `SHADOWMASK_PORT`          | `8080`     | the dev server's port on the host               |

## Provisioning

1. Checks that a Shadowmask server answers at `http://10.0.2.2:<SHADOWMASK_PORT>/health` (the
   host, through VirtualBox NAT).
2. Turns Smart App Control off, since it blocks unsigned binaries. This cannot be undone for that
   Windows install. If it is still enforced, run `vagrant reload --provision` with the same
   variables.
3. `build` only (`build.ps1`): installs Git and Visual Studio 2022 Build Tools with winget and
   Flutter (the version in `.github/workflows/build.yml`) into `C:\tools`, copies
   `clients/flutter` to `C:\tools\build\flutter`, builds there from clean and writes
   `artifacts/shadowmask-client-windows-<version>.zip`. The first run takes 30-60 minutes.
   On arm64, libmpv is extracted with Windows `tar.exe` unless 7-Zip is on `PATH`.
4. Stops a running client, unpacks the newest zip to `%LOCALAPPDATA%\Programs\shadowmask` and adds
   Start menu and Desktop shortcuts.

## Notes

- The VM is named `shadowmask-windows`. Its forwarded ports (WinRM, RDP) listen on `127.0.0.1`;
  a port another VM already holds is replaced from 2610-2700.
- On an arm64 host the guest is Windows on Arm and a `build` run produces an arm64 client. A
  release zip is x64 and runs there under emulation.
- The guest has no GPU, so hardware decoding cannot be checked here.
- The guest has an HD Audio device that plays through the host's default output.
