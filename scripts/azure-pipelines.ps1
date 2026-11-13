param(
    [string]$Workflow = "azure-pipelines.yml",
    [string]$Check = "*",
    [string[]]$Files = @(),
    [string]$Binary = "already-tested"
)

$ErrorActionPreference = "Stop"
Write-Host "##vso[task.setvariable variable=skip;isOutput=true]false"

if ($env:BUILD_REASON -ne "PullRequest" -or $env:BUILD_REPOSITORY_PROVIDER -ne "TfsGit") {
    exit 0
}

try {
    if (!$env:SYSTEM_ACCESSTOKEN -or !$env:SYSTEM_PULLREQUEST_PULLREQUESTID -or
        !$env:SYSTEM_TEAMPROJECTID -or !$env:BUILD_REPOSITORY_ID) {
        throw "Azure Repos pull-request context or access token is missing."
    }
    $collection = [uri]$env:SYSTEM_COLLECTIONURI
    if ($collection.Scheme -ne "https") {
        throw "The Azure DevOps collection URL must use HTTPS."
    }
    $url = "$($collection.AbsoluteUri.TrimEnd('/'))/$env:SYSTEM_TEAMPROJECTID/_apis/git/repositories/$env:BUILD_REPOSITORY_ID/pullrequests/$($env:SYSTEM_PULLREQUEST_PULLREQUESTID)?api-version=7.1"
    $pr = Invoke-RestMethod -Uri $url -Headers @{
        Authorization = "Bearer $env:SYSTEM_ACCESSTOKEN"
    } -TimeoutSec 30
    $arguments = @("ci", "--ci", "azure-pipelines", "--workflow", $Workflow, "--check", $Check)
    foreach ($pattern in $Files) {
        $arguments += @("--files", $pattern)
    }
    $output = $pr.description | & $Binary @arguments
    if ($LASTEXITCODE -eq 0 -and $output -is [string] -and
        $output -ceq "##vso[task.setvariable variable=skip;isOutput=true]true") {
        Write-Host "##vso[task.setvariable variable=skip;isOutput=true]true"
    }
} catch {
    Write-Host "already-tested: Azure verification unavailable; run CI normally."
}
exit 0
