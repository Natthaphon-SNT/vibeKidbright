$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot '../check-winget-token.ps1'
foreach ($value in @('public_repo, workflow', 'repo, workflow', ' workflow , public_repo ')) {
    & $script -ValidateHeaderOnly -ScopeHeader $value
}
foreach ($value in @('public_repo', 'repo', '', 'workflow')) {
    $rejected = $false
    try { & $script -ValidateHeaderOnly -ScopeHeader $value } catch { $rejected = $true }
    if (-not $rejected) { throw "Missing permission fixture was accepted: $value" }
}
Write-Host '7 offline token-scope cases passed. No credentials read or network requests made.'
