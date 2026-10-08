using Microsoft.UI.Xaml;

namespace Mailune.App;

/// <summary>Opens the window. The core binding is not generated on this branch.</summary>
public partial class App : Application
{
    private Window? window;

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        window = new MainWindow();
        window.Activate();
    }
}
