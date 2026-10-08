# Windows release

Mailune for Windows ships as a signed MSIX and a winget manifest. This tree does not contain a signing certificate. Packing and signing are the reusable workflow `pyrlyn/ci/.github/workflows/windows-sign.yml`.

## Package

`desktop/windows/packaging/AppxManifest.xml` is the layout description for one architecture.

- Identity name: `Pyrlyn.Mailune`
- Publisher: `CN=Mailune Placeholder`. That string is not a certificate. At pack time the layout's `Publisher` must be the subject of `WINDOWS_CERTIFICATE`, the secret the workflow signs with.
- Version: `0.1.0.0` (four parts). The winget package version is `0.1.0`.
- The x64 layout sets `ProcessorArchitecture` to `x64`. The arm64 layout is the same manifest with `arm64`.
- `Mailune.App` keeps `WindowsPackageType` at `None`. A local build stays unpackaged. The workflow packs the layout; `.github/workflows/ci.yml` is unchanged.

The layout directory passed as `layout` has this manifest at its root, `Mailune.exe`, and the logo files named in the manifest. Those images are part of the pack layout, not of this description.

## Signing

`windows-sign.yml` checks `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PWD`, runs `makeappx pack`, then `signtool`. A missing secret stops the run before a package is written. Call it once per architecture:

```yaml
jobs:
  sign:
    uses: pyrlyn/ci/.github/workflows/windows-sign.yml@<sha>
    permissions:
      contents: read
      actions: write
    with:
      layout: desktop/windows/layout-x64
      output: Mailune-x64.msix
    secrets: inherit
```

## Winget

`desktop/windows/winget/Pyrlyn.Mailune.yaml` is the singleton manifest. `InstallerUrl` and `InstallerSha256` are placeholders. The release that publishes the signed MSIX replaces them. Do not submit the placeholder digest to winget.
