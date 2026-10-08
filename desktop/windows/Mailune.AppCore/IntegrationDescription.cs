namespace Mailune.AppCore;

/// <summary>
/// What a toast and the taskbar badge would carry. No toast is posted from here.
/// </summary>
public sealed record IntegrationDescription(string ToastAction, int BadgeCount)
{
    public static IntegrationDescription Open() => new("open-thread", 1);
}
