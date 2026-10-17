# ADR 0009: Immutable tags and recoverable installer publication

Status: Accepted · 2026-10-17

GitHub Actions publishes the Windows x64 installer and its SHA-256 checksum from
an existing stable `vMAJOR.MINOR.PATCH` tag. Resolve the tag to a commit before
building and check every manifest and lockfile against its version. Only the
publishing job receives repository write permission.

A tag push normally starts publication. A push to `main` also recovers its current
package version when that tag exists in its history but has no published release.
Manual dispatch on `main` accepts an existing tag for an explicit retry. This
handles missed tag events without force-pushing or moving release tags.

Published assets remain unchanged on retries. Serialize publication for each tag,
recheck for an existing release immediately before publishing, and allow an
unfinished draft to resume. Failed release lookups fail the workflow rather than
being interpreted as absent releases.

These network operations happen only during distribution. The installed app
retains its offline behavior, with no update service or additional runtime network
requirement. Installers remain unsigned until signing is explicitly configured.

References: DESIGN §§1, 17; `docs/windows-release.md`.
