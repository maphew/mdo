# Draft Play Console declarations

These answers describe the current source. Re-check them against the exact
bundle uploaded to Play before attesting.

## Data safety

- Does the app collect or share any required user data type? **No.**
- Is all user data encrypted in transit? **Not applicable; the app does not
  transmit user data and has no Internet permission.**
- Can users request data deletion? **Not applicable; there is no account or
  server-side data.** Local settings and retained document permissions are
  deleted by clearing app storage or uninstalling.
- Privacy policy: `https://maphew.github.io/mdo/android-privacy.html`

## App content and access

- Ads: **No.**
- App access: **All functionality is available without login or special
  credentials.**
- Government, financial, health, VPN, news, or dating app: **No.**
- Broad file access (`MANAGE_EXTERNAL_STORAGE`): **Not requested.** Documents
  are chosen through Android's Storage Access Framework.
- User-generated content: mdo displays only local content selected by the
  user; it has no hosting, sharing network, or public content feed.
- Content rating: utility/productivity viewer; the app itself contains no
  violence, sexuality, gambling, controlled substances, or social features.

## Owner decisions still required

- Confirm the target age group. Recommended: general audience, not designed
  specifically for children.
- Confirm the public developer name, support email, website, and geographic
  distribution.
- Complete Play's exact questionnaire wording; Google requires the developer,
  not the build pipeline, to attest that these answers are accurate.
