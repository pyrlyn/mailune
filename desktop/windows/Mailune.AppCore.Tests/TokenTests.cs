using System.Xml.Linq;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class TokenTests
{
    [TestMethod]
    public void TheTokenDictionaryNamesTheColorKeys()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "Tokens.xaml");
        var document = XDocument.Load(path);
        XNamespace xaml = "http://schemas.microsoft.com/winfx/2006/xaml";
        var keys = document
            .Descendants()
            .Select(element => (string?)element.Attribute(xaml + "Key"))
            .Where(key => key is not null)
            .Cast<string>()
            .ToList();

        CollectionAssert.IsSubsetOf(new[] { "ColorInk", "ColorPaper", "ColorAccent" }, keys);
        CollectionAssert.IsSubsetOf(new[] { "FontFamilyBody", "FontSizeBody", "Space2", "Space4", "IconInbox" }, keys);
    }
}
