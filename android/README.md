# mdo for Android

The Android app opens local Markdown documents and renders them through the
same Rust library, sanitizer, bundled simple.css, and theme toggle as the mdo
desktop CLI. It requests no network permission.

## Install the preview

Download the signed ARM64 APK from the
[latest GitHub Release](https://github.com/maphew/mdo/releases/latest/download/mdo-android-arm64.apk),
open it on your Android device, and approve the prompt to allow installation
from that source. The preview supports 64-bit ARM devices running Android 6.0
or newer.

## Build a debug APK

Requirements:

- JDK 17
- Android SDK platform 36 and NDK `28.2.13676358`
- Rust with the `aarch64-linux-android` target
- [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk)

```bash
rustup target add aarch64-linux-android
cargo install cargo-ndk --locked
cd android
./gradlew assembleDebug
```

On Windows, use `gradlew.bat assembleDebug`. The APK is written to
`app/build/outputs/apk/debug/app-debug.apk` and supports 64-bit ARM
devices running Android 6.0 or newer.

## Use it

- Launch **mdo**, tap **Open**, and choose a Markdown document.
- Or open/share a Markdown document from Android's Files app and select
  **mdo**.

Documents are read through Android's Storage Access Framework, so the app
does not request broad storage access. Raw HTML is sanitized. Relative local
images are not yet resolved through document-provider URIs.

The app's privacy policy is
[`docs/android-privacy.md`](../docs/android-privacy.md). It has no Internet
permission, ads, analytics, accounts, or telemetry.

## Release packages

Tagged GitHub Releases publish the signed installable APK as
`mdo-android-arm64.apk` and the Play Store upload bundle as
`mdo-android-arm64.aab`. The AAB is not directly installable on a device.

Without signing environment variables, `assembleRelease` and `bundleRelease`
produce unsigned packages suitable for verification and F-Droid's own signing
process. To create signed packages, set:

```text
MDO_ANDROID_KEYSTORE=/absolute/path/to/mdo-upload.jks
MDO_ANDROID_KEYSTORE_PASSWORD=...
MDO_ANDROID_KEY_ALIAS=...
MDO_ANDROID_KEY_PASSWORD=...
```

Then run:

```bash
./gradlew assembleRelease bundleRelease
```

Never commit the keystore or passwords. GitHub release-secret setup is
documented in [`docs/maintaining.md`](../docs/maintaining.md).
