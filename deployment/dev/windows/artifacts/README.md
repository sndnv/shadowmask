# artifacts

Provisioning installs the newest `shadowmask-client-windows-*.zip` in this directory, by
modification time. A `build` run writes one here; a zip from a release works too, and so does one
from a `publish_branch.yml` run once unpacked from the downloaded `client-windows.zip`. The zips are
git-ignored.
