param([int]$AppProcessId,[ValidateSet('inspect','cancel','select')][string]$Action='inspect',[string]$Directory='')
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
using System.Collections.Generic;
public class WorkspaceDialog {
 public delegate bool Callback(IntPtr hwnd, IntPtr data);
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent,Callback callback,IntPtr data);
 [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr hwnd);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd,StringBuilder text,int max);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr hwnd,StringBuilder text,int max);
 [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr hwnd,int id);
 [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr hwnd,uint message,IntPtr wparam,string lparam);
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] public static extern IntPtr ReadText(IntPtr hwnd,uint message,IntPtr wparam,StringBuilder lparam);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd,uint message,IntPtr wparam,IntPtr lparam);
 public static object[] Children(IntPtr parent) {
  var items=new List<object>();
  EnumChildWindows(parent,(hwnd,data)=>{var text=new StringBuilder(512);var cls=new StringBuilder(256);GetWindowText(hwnd,text,text.Capacity);GetClassName(hwnd,cls,cls.Capacity);int id=GetDlgCtrlID(hwnd);if(id==1||id==2||id==1148||id==1152||cls.ToString()=="Edit") items.Add(new {hwnd=hwnd.ToInt64(),id=id,cls=cls.ToString(),name=text.ToString()});return true;},IntPtr.Zero);
  return items.ToArray();
 }
 public static IntPtr Child(IntPtr parent,int controlId) {
  IntPtr found=IntPtr.Zero;
  EnumChildWindows(parent,(hwnd,data)=>{if(GetDlgCtrlID(hwnd)==controlId){found=hwnd;return false;}return true;},IntPtr.Zero);
  return found;
 }
}
'@
$taskRoot=[System.Windows.Automation.AutomationElement]::RootElement
$taskProcessCondition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$AppProcessId)
$taskDialog=$null
for($taskAttempt=0;$taskAttempt -lt 60;$taskAttempt++) {
 $taskWindows=$taskRoot.FindAll([System.Windows.Automation.TreeScope]::Children,$taskProcessCondition)
 foreach($taskWindow in $taskWindows) { if($taskWindow.Current.ClassName -eq '#32770') {$taskDialog=$taskWindow;break} }
 if($taskDialog) {break}
 Start-Sleep -Milliseconds 100
}
if(-not $taskDialog) {throw 'No file-picker window found for the isolated application process.'}
$taskControls=$taskDialog.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
$taskRows=@()
foreach($taskControl in $taskControls) {
 if($taskControl.Current.ControlType -in @([System.Windows.Automation.ControlType]::Edit,[System.Windows.Automation.ControlType]::Button,[System.Windows.Automation.ControlType]::ComboBox)) {
  $taskRows+=@{name=$taskControl.Current.Name;id=$taskControl.Current.AutomationId;type=$taskControl.Current.ControlType.ProgrammaticName;enabled=$taskControl.Current.IsEnabled}
 }
}
$taskHwnd=[IntPtr]$taskDialog.Current.NativeWindowHandle
if($Action -eq 'inspect') { @{dialog=@{name=$taskDialog.Current.Name;hwnd=$taskHwnd.ToInt64()};native=[WorkspaceDialog]::Children($taskHwnd);controls=$taskRows} | ConvertTo-Json -Depth 5; exit 0 }
if($Action -eq 'cancel') {
 [WorkspaceDialog]::PostMessage($taskHwnd,0x111,[IntPtr]2,[WorkspaceDialog]::GetDlgItem($taskHwnd,2)) | Out-Null
 for($taskWait=0;$taskWait -lt 30 -and [WorkspaceDialog]::IsWindow($taskHwnd);$taskWait++) {Start-Sleep -Milliseconds 100}
 if([WorkspaceDialog]::IsWindow($taskHwnd)) {throw 'Native file picker did not close after Cancel.'}
 Write-Output 'Cancelled isolated file picker.';exit 0
}
if($Action -eq 'select') {
 $taskResolved=(Resolve-Path -LiteralPath $Directory -ErrorAction Stop).Path
 $taskPermitted=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../src-tauri/target/desktop-evaluation')).Path
 if(-not $taskResolved.StartsWith($taskPermitted+'\',[System.StringComparison]::OrdinalIgnoreCase)) {throw 'Picker automation is limited to generated desktop-evaluation fixtures.'}
 $taskEdit=[WorkspaceDialog]::Child($taskHwnd,1152)
 $taskSelect=[WorkspaceDialog]::Child($taskHwnd,1)
 if($taskEdit -eq [IntPtr]::Zero -or $taskSelect -eq [IntPtr]::Zero) {throw 'Native file-picker controls differ from the observed Windows control IDs.'}
 [WorkspaceDialog]::SendMessage($taskEdit,0xC,[IntPtr]::Zero,$taskResolved) | Out-Null
 $taskActual=New-Object System.Text.StringBuilder(32768)
 [WorkspaceDialog]::ReadText($taskEdit,0xD,[IntPtr]$taskActual.Capacity,$taskActual) | Out-Null
 if($taskActual.ToString() -ne $taskResolved) {throw 'Native folder text did not match the permitted scratch path.'}
 [WorkspaceDialog]::PostMessage($taskSelect,0xF5,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
 for($taskWait=0;$taskWait -lt 50 -and [WorkspaceDialog]::IsWindow($taskHwnd);$taskWait++) {Start-Sleep -Milliseconds 100}
 if([WorkspaceDialog]::IsWindow($taskHwnd)) {throw 'Native picker did not close after choosing the fixture folder.'}
 Write-Output 'Selected generated fixture folder.';exit 0
}
