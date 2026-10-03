using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text;

public static class MscServiceNative
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct CredUiInfo
    {
        public uint Size;
        public IntPtr Owner;
        public string Message;
        public string Caption;
        public IntPtr Banner;
    }

    [DllImport("credui.dll", CharSet = CharSet.Unicode, EntryPoint = "CredUIPromptForCredentialsW")]
    private static extern uint Prompt(ref CredUiInfo info, string target, IntPtr reserved,
        uint error, StringBuilder user, uint userSize, StringBuilder password,
        uint passwordSize, ref bool save, uint flags);

    public static string PromptPassword(string account, IntPtr owner)
    {
        var info = new CredUiInfo {
            Size = (uint)Marshal.SizeOf(typeof(CredUiInfo)), Owner = owner,
            Caption = "MSC 2 agent service",
            Message = "Enter the Windows password for " + account +
                ". Windows Hello PINs cannot register a service."
        };
        var user = new StringBuilder(account, 513);
        var password = new StringBuilder(257);
        bool save = false;
        try {
            // Always display a dialog and never persist its password; validate the account below.
            uint error = Prompt(ref info, "MSC 2 agent service", IntPtr.Zero, 0,
                user, 513, password, 257, ref save, 0x40000 | 0x80 | 0x2);
            if (error == 1223) throw new OperationCanceledException("Windows service installation was cancelled.");
            if (error != 0) throw new Win32Exception((int)error);
            if (!String.Equals(user.ToString(), account, StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("The service must run as the installing Windows account.");
            if (password.Length == 0) throw new InvalidOperationException("The Windows account password cannot be empty.");
            return password.ToString();
        } finally {
            for (int i = 0; i < password.Length; i++) password[i] = '\0';
            password.Clear();
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PolicyAttributes
    {
        public uint Length;
        public IntPtr Root, Name;
        public uint Attributes;
        public IntPtr Descriptor, Quality;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct UnicodeString
    {
        public ushort Length, MaximumLength;
        public IntPtr Buffer;
    }

    [DllImport("advapi32.dll")]
    private static extern uint LsaOpenPolicy(IntPtr system, ref PolicyAttributes attributes,
        uint access, out IntPtr policy);
    [DllImport("advapi32.dll")]
    private static extern uint LsaAddAccountRights(IntPtr policy, byte[] sid,
        ref UnicodeString rights, uint count);
    [DllImport("advapi32.dll")]
    private static extern uint LsaClose(IntPtr policy);
    [DllImport("advapi32.dll")]
    private static extern uint LsaNtStatusToWinError(uint status);

    private static SecurityIdentifier AccountSid(string account)
    {
        return (SecurityIdentifier)new NTAccount(account).Translate(typeof(SecurityIdentifier));
    }

    private static void Check(uint status)
    {
        if (status != 0) throw new Win32Exception((int)LsaNtStatusToWinError(status));
    }

    public static void GrantServiceLogon(string account)
    {
        var attributes = new PolicyAttributes { Length = (uint)Marshal.SizeOf(typeof(PolicyAttributes)) };
        IntPtr policy;
        Check(LsaOpenPolicy(IntPtr.Zero, ref attributes, 0x800 | 0x10, out policy));
        var right = new UnicodeString();
        try {
            var sid = AccountSid(account);
            var bytes = new byte[sid.BinaryLength];
            sid.GetBinaryForm(bytes, 0);
            const string name = "SeServiceLogonRight";
            right.Buffer = Marshal.StringToHGlobalUni(name);
            right.Length = (ushort)(name.Length * 2);
            right.MaximumLength = (ushort)(right.Length + 2);
            Check(LsaAddAccountRights(policy, bytes, ref right, 1));
        } finally {
            if (right.Buffer != IntPtr.Zero) Marshal.FreeHGlobal(right.Buffer);
            LsaClose(policy);
        }
    }

    public static string AllowOwnerControl(string descriptor, string account)
    {
        var security = new RawSecurityDescriptor(descriptor);
        // Query status, start and stop; no configuration or deletion permission.
        security.DiscretionaryAcl.InsertAce(security.DiscretionaryAcl.Count,
            new CommonAce(AceFlags.None, AceQualifier.AccessAllowed, 0x34, AccountSid(account), false, null));
        return security.GetSddlForm(AccessControlSections.All);
    }
}
