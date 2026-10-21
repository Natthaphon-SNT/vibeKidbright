# WinGet release setup

The screenshot's HTTP 422 is a missing permission on WINGET_TOKEN: syncing the winget-pkgs fork also updates upstream workflow files. winget-releaser v2 requires a classic PAT with public_repo and workflow scopes. Changing GITHUB_TOKEN permissions does not grant scopes to that separate PAT.

1. In GitHub Settings → Developer settings → Personal access tokens (classic), update/create the release account's token with public_repo and workflow, appropriate expiration and organization authorization.
2. Update the repository's Settings → Secrets and variables → Actions → WINGET_TOKEN if its value changes. Never paste the token in chat, source, command arguments or logs.
3. The account must own/have access to its microsoft/winget-pkgs fork. Natthaphon-SNT.vibeKidbright must already have an initial accepted version upstream; this updater cannot bootstrap the first submission.
4. After an authorized maintainer commits the updated workflow, use Actions → Submit to winget on Release → Run workflow with the original published release tag. This runs the new preflight instead of the old workflow revision. It does not rebuild or replace installers. Only x64 NSIS setup assets are selected.

The preflight checks scope headers through the GitHub API using a secret environment variable. Offline tests use no credentials/network. This local change has not changed repository secrets, submitted a WinGet PR or validated a live Actions run.

The action also documents manually syncing the fork in GitHub's web UI as a workaround when workflow scope is unavailable; later upstream workflow changes can require this again.

Source: [WinGet Releaser prerequisites](https://github.com/vedantmgoyal9/winget-releaser#readme).
