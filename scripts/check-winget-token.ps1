param([switch]$ValidateHeaderOnly, [string]$ScopeHeader = '')
$ErrorActionPreference = 'Stop'
function Assert-WingetScopes([string]$HeaderValue) {
    $scopes = @($HeaderValue.Split(',') | ForEach-Object { $_.Trim() })
    if (-not (($scopes -contains 'public_repo') -or ($scopes -contains 'repo'))) {
        throw 'WINGET_TOKEN must be a classic PAT with public_repo and workflow scopes. Fine-grained PATs are not supported by winget-releaser v2. Update the repository Actions secret; never paste the token in logs.'
    }
    if (-not ($scopes -contains 'workflow')) {
        throw 'WINGET_TOKEN is missing workflow scope. Fork synchronization updates upstream .github/workflows files and fails with HTTP 422 without it. Add workflow to the classic PAT (keep public_repo), update the Actions secret if its value changes, then run the updated workflow with the original release tag. GITHUB_TOKEN permissions do not change this PAT.'
    }
}
# Offline regression entry: no credential access or network request.
if ($ValidateHeaderOnly) { Assert-WingetScopes $ScopeHeader; return }
if ([string]::IsNullOrWhiteSpace($env:GH_TOKEN)) {
    throw 'Add a classic PAT with public_repo and workflow as the repository Actions secret WINGET_TOKEN.'
}
# Capture, never print the user response or token; authentication is supplied
# through GH_TOKEN rather than interpolated into executable shell source.
$response = (gh api --include user 2>&1) -join "`n"
if ($LASTEXITCODE -ne 0) { throw 'Cannot validate WINGET_TOKEN. Check expiration, revocation and organization authorization.' }
$match = [regex]::Match($response, '(?im)^x-oauth-scopes:[ \t]*([^\r\n]*)')
Assert-WingetScopes $match.Groups[1].Value
Write-Host 'Classic token scopes validated for WinGet submission and fork synchronization.'
