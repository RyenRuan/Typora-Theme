$ErrorActionPreference = 'Stop'

$createdNew = $false
$mutex = [System.Threading.Mutex]::new($true, 'Local\TyporaCornerQuotesHook_v1', [ref]$createdNew)
if (-not $createdNew) {
    exit 0
}

Add-Type -AssemblyName System.Windows.Forms

$source = @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Windows.Forms;

public static class TyporaCornerQuotesHook
{
    private const int WH_KEYBOARD_LL = 13;
    private const int WM_KEYDOWN = 0x0100;
    private const int WM_KEYUP = 0x0101;
    private const int WM_SYSKEYDOWN = 0x0104;
    private const int WM_SYSKEYUP = 0x0105;
    private const int VK_OEM_7 = 0xDE;
    private const int VK_SHIFT = 0x10;
    private const int VK_CONTROL = 0x11;
    private const int VK_MENU = 0x12;
    private const int VK_LEFT = 0x25;
    private const int VK_INSERT = 0x2D;
    private const int VK_V = 0x56;
    private const uint KEYEVENTF_EXTENDEDKEY = 0x0001;
    private const uint KEYEVENTF_KEYUP = 0x0002;
    private const uint KEYEVENTF_UNICODE = 0x0004;
    private const int INPUT_KEYBOARD = 1;
    private const uint IME_CMODE_NATIVE = 0x0001;
    private const uint WM_IME_CONTROL = 0x0283;
    private const uint IMC_GETCONVERSIONMODE = 0x0001;
    private const uint SMTO_ABORTIFHUNG = 0x0002;

    private static bool suppressQuoteKeyUp = false;
    private static bool suppressPasteKeyUp = false;
    private static bool sendingReplacementPaste = false;
    private static IDataObject clipboardBeforePaste = null;
    private static uint replacementClipboardSequence = 0;
    private static Timer clipboardRestoreTimer = null;
    private static IntPtr hookId = IntPtr.Zero;
    private static readonly LowLevelKeyboardProc hookProc = HookCallback;

    [StructLayout(LayoutKind.Sequential)]
    private struct KBDLLHOOKSTRUCT
    {
        public uint vkCode;
        public uint scanCode;
        public uint flags;
        public uint time;
        public UIntPtr dwExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct INPUT
    {
        public int type;
        public INPUTUNION data;
    }

    [StructLayout(LayoutKind.Explicit)]
    private struct INPUTUNION
    {
        [FieldOffset(0)]
        public MOUSEINPUT mouse;

        [FieldOffset(0)]
        public KEYBDINPUT keyboard;

        [FieldOffset(0)]
        public HARDWAREINPUT hardware;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MOUSEINPUT
    {
        public int dx;
        public int dy;
        public uint mouseData;
        public uint flags;
        public uint time;
        public UIntPtr extraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct KEYBDINPUT
    {
        public ushort virtualKey;
        public ushort scanCode;
        public uint flags;
        public uint time;
        public UIntPtr extraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct HARDWAREINPUT
    {
        public uint message;
        public ushort parameterLow;
        public ushort parameterHigh;
    }

    private delegate IntPtr LowLevelKeyboardProc(int code, IntPtr message, IntPtr data);

    public static void Run()
    {
        hookId = SetHook(hookProc);
        if (hookId == IntPtr.Zero)
            throw new InvalidOperationException("Unable to install keyboard hook.");

        Application.ApplicationExit += delegate
        {
            if (hookId != IntPtr.Zero)
                UnhookWindowsHookEx(hookId);
        };
        Application.Run();
    }

    private static IntPtr SetHook(LowLevelKeyboardProc proc)
    {
        using (Process current = Process.GetCurrentProcess())
        using (ProcessModule module = current.MainModule)
        {
            return SetWindowsHookEx(WH_KEYBOARD_LL, proc, GetModuleHandle(module.ModuleName), 0);
        }
    }

    private static IntPtr HookCallback(int code, IntPtr message, IntPtr data)
    {
        if (code < 0)
            return CallNextHookEx(hookId, code, message, data);

        KBDLLHOOKSTRUCT key = Marshal.PtrToStructure<KBDLLHOOKSTRUCT>(data);
        if (sendingReplacementPaste)
            return CallNextHookEx(hookId, code, message, data);

        bool isPasteKey = (key.vkCode == VK_V && IsKeyDown(VK_CONTROL)) ||
                          (key.vkCode == VK_INSERT && IsKeyDown(VK_SHIFT));
        bool isQuoteKey = key.vkCode == VK_OEM_7;
        if ((!isQuoteKey && !isPasteKey) || !IsTyporaForeground() || !IsChineseInputMode())
            return CallNextHookEx(hookId, code, message, data);

        bool isDown = message == (IntPtr)WM_KEYDOWN || message == (IntPtr)WM_SYSKEYDOWN;
        bool isUp = message == (IntPtr)WM_KEYUP || message == (IntPtr)WM_SYSKEYUP;
        if (!isDown && !isUp)
            return CallNextHookEx(hookId, code, message, data);

        if (isUp)
        {
            if (isPasteKey && suppressPasteKeyUp)
            {
                suppressPasteKeyUp = false;
                return (IntPtr)1;
            }

            if (suppressQuoteKeyUp)
            {
                suppressQuoteKeyUp = false;
                return (IntPtr)1;
            }
            return CallNextHookEx(hookId, code, message, data);
        }

        if (isDown)
        {
            if (isPasteKey)
            {
                suppressPasteKeyUp = PasteWithConvertedChineseQuotes(key.vkCode == VK_INSERT);
                if (suppressPasteKeyUp)
                    return (IntPtr)1;
                return CallNextHookEx(hookId, code, message, data);
            }

            bool shift = IsKeyDown(VK_SHIFT);
            bool control = IsKeyDown(VK_CONTROL);
            if (control)
            {
                suppressQuoteKeyUp = SendUnicode(shift ? '"' : '\'');
            }
            else if (shift)
            {
                suppressQuoteKeyUp = SendUnicodePair('\u300C', '\u300D', true);
            }
            else
            {
                suppressQuoteKeyUp = SendUnicodePair('\u300E', '\u300F', false);
            }

            if (suppressQuoteKeyUp)
                return (IntPtr)1;
        }

        return CallNextHookEx(hookId, code, message, data);
    }

    private static bool PasteWithConvertedChineseQuotes(bool useShiftInsert)
    {
        try
        {
            if (!Clipboard.ContainsText(TextDataFormat.UnicodeText))
                return false;

            string originalText = Clipboard.GetText(TextDataFormat.UnicodeText);
            string convertedText = ConvertChineseQuotes(originalText);
            if (string.Equals(originalText, convertedText, StringComparison.Ordinal))
                return false;

            clipboardBeforePaste = SnapshotClipboard();
            Clipboard.SetDataObject(CreateConvertedClipboard(clipboardBeforePaste, convertedText), true);
            replacementClipboardSequence = GetClipboardSequenceNumber();

            sendingReplacementPaste = true;
            try
            {
                return SendPasteShortcut(useShiftInsert);
            }
            finally
            {
                sendingReplacementPaste = false;
                ScheduleClipboardRestore();
            }
        }
        catch
        {
            clipboardBeforePaste = null;
            return false;
        }
    }

    private static string ConvertChineseQuotes(string text)
    {
        if (string.IsNullOrEmpty(text))
            return text;

        return text
            .Replace('\u201C', '\u300C')
            .Replace('\u201D', '\u300D')
            .Replace('\u2018', '\u300E')
            .Replace('\u2019', '\u300F');
    }

    private static IDataObject SnapshotClipboard()
    {
        IDataObject source = Clipboard.GetDataObject();
        DataObject snapshot = new DataObject();
        if (source == null)
            return snapshot;

        foreach (string format in source.GetFormats(false))
        {
            try
            {
                object data = source.GetData(format, false);
                if (data != null)
                    snapshot.SetData(format, false, data);
            }
            catch
            {
                // A clipboard owner may advertise a format that cannot be read.
                // Keep all accessible formats and skip only the failing one.
            }
        }

        return snapshot;
    }

    private static IDataObject CreateConvertedClipboard(IDataObject source, string convertedUnicodeText)
    {
        DataObject converted = new DataObject();
        if (source != null)
        {
            foreach (string format in source.GetFormats(false))
            {
                try
                {
                    object data = source.GetData(format, false);
                    if (data is string)
                    {
                        string stringData = (string)data;
                        if (string.Equals(format, DataFormats.Html, StringComparison.OrdinalIgnoreCase))
                            data = ConvertChineseQuotesInHtml(stringData);
                        else if (string.Equals(format, DataFormats.UnicodeText, StringComparison.OrdinalIgnoreCase) ||
                                 string.Equals(format, DataFormats.Text, StringComparison.OrdinalIgnoreCase) ||
                                 string.Equals(format, DataFormats.OemText, StringComparison.OrdinalIgnoreCase) ||
                                 string.Equals(format, DataFormats.StringFormat, StringComparison.OrdinalIgnoreCase))
                            data = ConvertChineseQuotes(stringData);
                    }

                    if (data != null)
                        converted.SetData(format, false, data);
                }
                catch
                {
                    // Preserve every accessible clipboard format. Typora can
                    // still choose HTML when pasting rich web content.
                }
            }
        }

        converted.SetData(DataFormats.UnicodeText, true, convertedUnicodeText);
        return converted;
    }

    private static string ConvertChineseQuotesInHtml(string html)
    {
        if (string.IsNullOrEmpty(html))
            return html;

        int startHtml = GetClipboardHtmlOffset(html, "StartHTML");
        int endHtml = GetClipboardHtmlOffset(html, "EndHTML");
        int startFragment = GetClipboardHtmlOffset(html, "StartFragment");
        int endFragment = GetClipboardHtmlOffset(html, "EndFragment");

        int startHtmlChar = ByteOffsetToCharIndex(html, startHtml);
        int endHtmlChar = ByteOffsetToCharIndex(html, endHtml);
        int startFragmentChar = ByteOffsetToCharIndex(html, startFragment);
        int endFragmentChar = ByteOffsetToCharIndex(html, endFragment);

        string converted = ReplaceHtmlQuoteCharacters(html);

        // CF_HTML stores UTF-8 byte offsets in its header. Entity replacement
        // changes byte length, so rebuild those offsets before Typora pastes it.
        if (startHtmlChar >= 0 && endHtmlChar >= 0 &&
            startFragmentChar >= 0 && endFragmentChar >= 0)
        {
            int newStartHtml = ConvertedUtf8PrefixLength(html, startHtmlChar);
            int newEndHtml = ConvertedUtf8PrefixLength(html, endHtmlChar);
            int newStartFragment = ConvertedUtf8PrefixLength(html, startFragmentChar);
            int newEndFragment = ConvertedUtf8PrefixLength(html, endFragmentChar);

            converted = ReplaceClipboardHtmlOffset(converted, "StartHTML", newStartHtml);
            converted = ReplaceClipboardHtmlOffset(converted, "EndHTML", newEndHtml);
            converted = ReplaceClipboardHtmlOffset(converted, "StartFragment", newStartFragment);
            converted = ReplaceClipboardHtmlOffset(converted, "EndFragment", newEndFragment);
        }

        return converted;
    }

    private static string ReplaceHtmlQuoteCharacters(string html)
    {
        return ConvertChineseQuotes(html)
            .Replace("&ldquo;", "\u300C")
            .Replace("&rdquo;", "\u300D")
            .Replace("&lsquo;", "\u300E")
            .Replace("&rsquo;", "\u300F")
            .Replace("&#8220;", "\u300C")
            .Replace("&#8221;", "\u300D")
            .Replace("&#8216;", "\u300E")
            .Replace("&#8217;", "\u300F")
            .Replace("&#x201C;", "\u300C")
            .Replace("&#x201D;", "\u300D")
            .Replace("&#x2018;", "\u300E")
            .Replace("&#x2019;", "\u300F")
            .Replace("&#x201c;", "\u300C")
            .Replace("&#x201d;", "\u300D");
    }

    private static int GetClipboardHtmlOffset(string html, string fieldName)
    {
        Match match = Regex.Match(
            html,
            "(?im)^" + Regex.Escape(fieldName) + ":(?<value>[0-9]+)");
        int value;
        return match.Success && int.TryParse(match.Groups["value"].Value, out value)
            ? value
            : -1;
    }

    private static int ByteOffsetToCharIndex(string text, int utf8ByteOffset)
    {
        if (utf8ByteOffset < 0)
            return -1;

        byte[] bytes = Encoding.UTF8.GetBytes(text);
        if (utf8ByteOffset > bytes.Length)
            return -1;

        return Encoding.UTF8.GetCharCount(bytes, 0, utf8ByteOffset);
    }

    private static int ConvertedUtf8PrefixLength(string original, int charIndex)
    {
        if (charIndex < 0 || charIndex > original.Length)
            return -1;

        string convertedPrefix = ReplaceHtmlQuoteCharacters(original.Substring(0, charIndex));
        return Encoding.UTF8.GetByteCount(convertedPrefix);
    }

    private static string ReplaceClipboardHtmlOffset(string html, string fieldName, int value)
    {
        Regex fieldPattern = new Regex(
            "^(" + Regex.Escape(fieldName) + ":)(?<value>[0-9]+)",
            RegexOptions.IgnoreCase | RegexOptions.Multiline);
        return fieldPattern.Replace(
            html,
            delegate(Match match)
            {
                int width = match.Groups["value"].Value.Length;
                return match.Groups[1].Value + value.ToString("D" + width);
            },
            1);
    }

    private static void ScheduleClipboardRestore()
    {
        if (clipboardRestoreTimer == null)
        {
            clipboardRestoreTimer = new Timer();
            clipboardRestoreTimer.Interval = 700;
            clipboardRestoreTimer.Tick += delegate
            {
                clipboardRestoreTimer.Stop();
                try
                {
                    if (clipboardBeforePaste != null &&
                        GetClipboardSequenceNumber() == replacementClipboardSequence)
                    {
                        Clipboard.SetDataObject(clipboardBeforePaste, true);
                    }
                }
                catch
                {
                    // Never interrupt typing if another process temporarily locks
                    // the clipboard. The pasted text has already been delivered.
                }
                finally
                {
                    clipboardBeforePaste = null;
                    replacementClipboardSequence = 0;
                }
            };
        }

        clipboardRestoreTimer.Stop();
        clipboardRestoreTimer.Start();
    }

    private static bool SendPasteShortcut(bool useShiftInsert)
    {
        int virtualKey = useShiftInsert ? VK_INSERT : VK_V;
        INPUT[] inputs = new INPUT[2];
        inputs[0].type = INPUT_KEYBOARD;
        inputs[0].data.keyboard.virtualKey = (ushort)virtualKey;
        inputs[0].data.keyboard.flags = useShiftInsert ? KEYEVENTF_EXTENDEDKEY : 0;
        inputs[1].type = INPUT_KEYBOARD;
        inputs[1].data.keyboard.virtualKey = (ushort)virtualKey;
        inputs[1].data.keyboard.flags = (useShiftInsert ? KEYEVENTF_EXTENDEDKEY : 0) | KEYEVENTF_KEYUP;
        return SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(INPUT))) == inputs.Length;
    }

    private static bool IsTyporaForeground()
    {
        IntPtr foreground = GetForegroundWindow();
        if (foreground == IntPtr.Zero)
            return false;

        uint processId;
        GetWindowThreadProcessId(foreground, out processId);
        if (processId == 0)
            return false;

        try
        {
            using (Process process = Process.GetProcessById((int)processId))
                return string.Equals(process.ProcessName, "Typora", StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return false;
        }
    }

    private static bool IsChineseInputMode()
    {
        IntPtr foreground = GetForegroundWindow();
        if (foreground == IntPtr.Zero)
            return false;

        uint processId;
        uint threadId = GetWindowThreadProcessId(foreground, out processId);
        IntPtr keyboardLayout = GetKeyboardLayout(threadId);
        int languageId = unchecked((int)((long)keyboardLayout & 0xFFFF));
        int primaryLanguage = languageId & 0x03FF;
        if (primaryLanguage != 0x0004)
            return false;

        // Chromium windows (including Typora) commonly expose no HIMC on the
        // top-level window. Query the IME's own window first; Microsoft Pinyin
        // keeps the Chinese/English toggle in the IME_CMODE_NATIVE bit here.
        IntPtr imeWindow = ImmGetDefaultIMEWnd(foreground);
        if (imeWindow != IntPtr.Zero)
        {
            IntPtr conversionResult;
            if (SendMessageTimeout(
                    imeWindow,
                    WM_IME_CONTROL,
                    new IntPtr(IMC_GETCONVERSIONMODE),
                    IntPtr.Zero,
                    SMTO_ABORTIFHUNG,
                    100,
                    out conversionResult) != IntPtr.Zero)
            {
                return (conversionResult.ToInt64() & IME_CMODE_NATIVE) != 0;
            }
        }

        // Legacy fallback for applications that expose an input context.
        IntPtr inputContext = ImmGetContext(foreground);
        if (inputContext == IntPtr.Zero)
            return false;

        try
        {
            uint conversionMode;
            uint sentenceMode;
            if (ImmGetConversionStatus(inputContext, out conversionMode, out sentenceMode))
                return (conversionMode & IME_CMODE_NATIVE) != 0;

            return ImmGetOpenStatus(inputContext);
        }
        finally
        {
            ImmReleaseContext(foreground, inputContext);
        }
    }

    private static bool IsKeyDown(int virtualKey)
    {
        return (GetAsyncKeyState(virtualKey) & 0x8000) != 0;
    }

    private static bool SendUnicode(char character)
    {
        INPUT[] inputs = new INPUT[2];
        inputs[0].type = INPUT_KEYBOARD;
        inputs[0].data.keyboard.scanCode = character;
        inputs[0].data.keyboard.flags = KEYEVENTF_UNICODE;
        inputs[1].type = INPUT_KEYBOARD;
        inputs[1].data.keyboard.scanCode = character;
        inputs[1].data.keyboard.flags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
        return SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(INPUT))) == inputs.Length;
    }

    private static bool SendUnicodePair(char opening, char closing, bool shiftIsPhysicallyDown)
    {
        // Type both characters, then move the caret back between them. For the
        // double-quote shortcut, temporarily release Shift so Left Arrow moves
        // the caret instead of selecting the closing character.
        INPUT[] inputs = new INPUT[shiftIsPhysicallyDown ? 8 : 6];
        int index = 0;

        inputs[index].type = INPUT_KEYBOARD;
        inputs[index].data.keyboard.scanCode = opening;
        inputs[index++].data.keyboard.flags = KEYEVENTF_UNICODE;
        inputs[index].type = INPUT_KEYBOARD;
        inputs[index].data.keyboard.scanCode = opening;
        inputs[index++].data.keyboard.flags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;

        inputs[index].type = INPUT_KEYBOARD;
        inputs[index].data.keyboard.scanCode = closing;
        inputs[index++].data.keyboard.flags = KEYEVENTF_UNICODE;
        inputs[index].type = INPUT_KEYBOARD;
        inputs[index].data.keyboard.scanCode = closing;
        inputs[index++].data.keyboard.flags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;

        if (shiftIsPhysicallyDown)
        {
            inputs[index].type = INPUT_KEYBOARD;
            inputs[index].data.keyboard.virtualKey = VK_SHIFT;
            inputs[index++].data.keyboard.flags = KEYEVENTF_KEYUP;
        }

        inputs[index].type = INPUT_KEYBOARD;
        inputs[index].data.keyboard.virtualKey = VK_LEFT;
        inputs[index++].data.keyboard.flags = KEYEVENTF_EXTENDEDKEY;
        inputs[index].type = INPUT_KEYBOARD;
        inputs[index].data.keyboard.virtualKey = VK_LEFT;
        inputs[index++].data.keyboard.flags = KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP;

        if (shiftIsPhysicallyDown)
        {
            inputs[index].type = INPUT_KEYBOARD;
            inputs[index].data.keyboard.virtualKey = VK_SHIFT;
            inputs[index++].data.keyboard.flags = 0;
        }

        return SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(INPUT))) == inputs.Length;
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetWindowsHookEx(int hookId, LowLevelKeyboardProc callback, IntPtr module, uint threadId);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool UnhookWindowsHookEx(IntPtr hookId);

    [DllImport("user32.dll")]
    private static extern IntPtr CallNextHookEx(IntPtr hookId, int code, IntPtr message, IntPtr data);

    [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    private static extern IntPtr GetModuleHandle(string moduleName);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll")]
    private static extern IntPtr GetKeyboardLayout(uint threadId);

    [DllImport("imm32.dll")]
    private static extern IntPtr ImmGetContext(IntPtr window);

    [DllImport("imm32.dll")]
    private static extern bool ImmReleaseContext(IntPtr window, IntPtr inputContext);

    [DllImport("imm32.dll")]
    private static extern bool ImmGetOpenStatus(IntPtr inputContext);

    [DllImport("imm32.dll")]
    private static extern bool ImmGetConversionStatus(IntPtr inputContext, out uint conversionMode, out uint sentenceMode);

    [DllImport("imm32.dll")]
    private static extern IntPtr ImmGetDefaultIMEWnd(IntPtr window);

    [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam,
        uint flags,
        uint timeout,
        out IntPtr result);

    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int virtualKey);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint inputCount, INPUT[] inputs, int inputSize);

    [DllImport("user32.dll")]
    private static extern uint GetClipboardSequenceNumber();
}
'@

Add-Type -TypeDefinition $source -ReferencedAssemblies 'System.Windows.Forms'

$statusPath = Join-Path $PSScriptRoot 'typora-corner-quotes.status.txt'
$status = @(
    'RUNNING'
    "ProcessId=$PID"
    "Started=$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
    'Scope=Typora.exe foreground window only'
    'Shift+Quote=U+300C + caret + U+300D'
    'Quote=U+300E + caret + U+300F'
    'Ctrl+Shift+Quote=raw double quote'
    'Ctrl+Quote=raw single quote'
    'Chinese paste: U+201C/U+201D -> U+300C/U+300D; U+2018/U+2019 -> U+300E/U+300F'
) -join [Environment]::NewLine
[System.IO.File]::WriteAllText($statusPath, $status, [System.Text.UTF8Encoding]::new($false))

try {
    [TyporaCornerQuotesHook]::Run()
}
finally {
    $mutex.ReleaseMutex()
    $mutex.Dispose()
}
