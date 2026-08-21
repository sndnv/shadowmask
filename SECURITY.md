# Security Policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 1.x.x   | Yes       |

Fixes land on `main` and ship in the next release. They are not backported to older tags or images.

## Reporting a vulnerability

Report vulnerabilities privately through GitHub's private vulnerability reporting, from the
[Security tab](https://github.com/sndnv/shadowmask/security/advisories/new) of this repository.

Please do not open a public issue for a suspected vulnerability.

Include the version or commit, the configuration involved, and the steps to reproduce.

## Scope

In scope: authentication and session handling, stream token issuance and validation, role-based
access control, the TLS termination path, the remote content fetch path including the cookies file
it accepts, and any path that lets one account read another account's data.

Out of scope: vulnerabilities in third-party dependencies with no Shadowmask-specific exploit path
(report those upstream), and issues that require an operator to have already granted administrator
access.
