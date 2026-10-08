namespace Mailune.AppCore;

/// <summary>
/// A UI scenario the shell can name. UI Automation is not run here:
/// the WinUI compiler is absent on this machine.
/// </summary>
public sealed record UiScenario(string Subject)
{
    public static UiScenario Open() => new("Thursday");
}
