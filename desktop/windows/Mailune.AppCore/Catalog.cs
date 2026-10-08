namespace Mailune.AppCore;

/// <summary>
/// Shell strings. A missing entry in another language uses the English string.
/// </summary>
public static class Catalog
{
    private static readonly Dictionary<string, string> English = new()
    {
        ["Inbox"] = "Inbox",
        ["Settings"] = "Settings",
        ["Compose"] = "Compose",
        ["Send"] = "Send",
    };

    private static readonly Dictionary<string, string> Russian = new()
    {
        ["Inbox"] = "Входящие",
        ["Settings"] = "Настройки",
        ["Compose"] = "Написать",
    };

    /// <summary>
    /// Looks up <paramref name="key"/> in <paramref name="language"/>.
    /// Russian falls back to English when the key is absent.
    /// </summary>
    public static string Text(string language, string key)
    {
        if (language == "ru" && Russian.TryGetValue(key, out var russian))
        {
            return russian;
        }

        if (English.TryGetValue(key, out var english))
        {
            return english;
        }

        return key;
    }
}
