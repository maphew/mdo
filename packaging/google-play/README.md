# Google Play preparation

The Android application ID is `io.github.maphew.mdo`. Treat it as permanent:
changing it creates a different Play application and breaks updates.

## Prepared in this repository

- Play-required Android App Bundle (`bundleRelease`)
- optional environment-based release signing for local and CI builds
- Fastlane/Triple-T listing copy under `android/fastlane/metadata/android/`
- 512×512 store icon and 1024×500 feature graphic, with editable SVG sources
  under `packaging/google-play/assets/`
- public privacy policy at `https://maphew.github.io/mdo/android-privacy.html`
- draft Data safety and policy answers in `data-safety.md`
- target SDK 36 and 64-bit ARM native library

## Human-owned setup

1. Choose a Play Console Personal or Organization developer account and
   complete Google's identity, contact, payment, and device verification.
2. Generate and securely back up a long-lived upload keystore. Do not commit
   it. Configure the four GitHub secrets documented in `docs/maintaining.md`,
   then enable the `ANDROID_RELEASE_SIGNING_ENABLED` repository variable.
3. Create the app in Play Console with application ID
   `io.github.maphew.mdo`, default language English (United States), app type
   App, category Productivity, and free pricing.
4. Upload the signed `mdo-android-arm64.aab` to Internal testing and enroll in
   Play App Signing. Keep Google's app-signing key separate from the upload
   key when offered.
5. Add at least two accurate phone screenshots under
   `android/fastlane/metadata/android/en-US/images/phoneScreenshots/`. Do not
   submit mock screenshots as real UI.
6. Review and submit the privacy, Data safety, target-audience, content-rating,
   app-access, ads, and store-contact declarations.
7. If this is a new Personal account, run the required closed test before
   requesting production access.

Signing-key creation, account enrollment, identity verification, policy
attestations, and final publication must be performed by the account owner.
