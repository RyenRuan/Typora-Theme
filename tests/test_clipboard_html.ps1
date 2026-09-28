$ErrorActionPreference = 'Stop'
$path = 'D:\Agent\ToolsDevelop\Typora-Theme\scripts\typora-corner-quotes.ps1'
$script = Get-Content -LiteralPath $path -Raw
$match = [regex]::Match($script, "\`$source = @'\r?\n(?<source>[\s\S]*?)\r?\n'@")
if (-not $match.Success) { throw 'C# source not found' }
Add-Type -TypeDefinition $match.Groups['source'].Value -ReferencedAssemblies 'System.Windows.Forms'
$type = [AppDomain]::CurrentDomain.GetAssemblies() | ForEach-Object { $_.GetType('TyporaCornerQuotesHook') } | Where-Object { $_ } | Select-Object -First 1
$method = $type.GetMethod('ConvertChineseQuotesInHtml', [Reflection.BindingFlags]::NonPublic -bor [Reflection.BindingFlags]::Static)
$header = "Version:1.0`r`nStartHTML:##########`r`nEndHTML:##########`r`nStartFragment:##########`r`nEndFragment:##########`r`n`r`n"
$bodyPrefix = '<html><body><!--StartFragment-->'
$fragment = '<thead><tr><td>中文</td></tr></thead>'
$bodySuffix = '<!--EndFragment--></body></html>'
$html = $header + $bodyPrefix + $fragment + $bodySuffix
$enc = [Text.Encoding]::UTF8
$startHtml = $enc.GetByteCount($header)
$endHtml = $enc.GetByteCount($html)
$startFragment = $enc.GetByteCount($header + $bodyPrefix)
$endFragment = $enc.GetByteCount($header + $bodyPrefix + $fragment)
$html = $html.Replace('StartHTML:##########', ('StartHTML:{0:D10}' -f $startHtml))
$html = $html.Replace('EndHTML:##########', ('EndHTML:{0:D10}' -f $endHtml))
$html = $html.Replace('StartFragment:##########', ('StartFragment:{0:D10}' -f $startFragment))
$html = $html.Replace('EndFragment:##########', ('EndFragment:{0:D10}' -f $endFragment))
$output = [string]$method.Invoke($null, @($html))
if (-not $output.Contains('<table>')) { throw 'Missing opening table wrapper' }
if (-not $output.Contains('</table>')) { throw 'Missing closing table wrapper' }
if ($output.Contains([char]0xFFFD)) { throw 'Replacement character found' }
$offsets = @{}
foreach ($line in ($output -split "`r?`n")) {
    if ($line -match '^(StartHTML|EndHTML|StartFragment|EndFragment):([0-9]+)') { $offsets[$matches[1]] = [int]$matches[2] }
}
$utf8 = [Text.Encoding]::UTF8.GetBytes($output)
foreach ($key in $offsets.Keys) { if ($offsets[$key] -gt $utf8.Length) { throw "$key exceeds output length" } }
"TEST_OK length=$($utf8.Length) offsets=$($offsets | ConvertTo-Json -Compress)"
