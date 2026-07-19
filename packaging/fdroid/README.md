# F-Droid preparation

`io.github.maphew.mdo.yml` is a working draft for the upstream `fdroiddata`
repository. mdo appears compatible with the main F-Droid inclusion policy:
all source and dependencies are FLOSS, the app builds with Gradle/Rust/NDK,
and it has no ads, tracking, proprietary service dependency, or network
requirement.

The draft is deliberately disabled until the first tagged release containing
the Android source. At that release:

1. replace the preview build block with the tag's full commit SHA, literal
   `versionName`, and `versionCode`;
2. remove `Disabled` and the preview `MaintainerNotes` text;
3. update `CurrentVersion` and `CurrentVersionCode`;
4. add two accurate phone screenshots under the upstream Fastlane metadata;
5. run `fdroid rewritemeta`, `fdroid lint`, and an isolated `fdroid build`;
6. fork `fdroid/fdroiddata` on GitLab and open a `New App` merge request.

F-Droid will build and sign its APK independently. The Rust renderer is built
from source; no native `.so` file is committed or downloaded.
