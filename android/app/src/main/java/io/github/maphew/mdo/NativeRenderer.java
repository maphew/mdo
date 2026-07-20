package io.github.maphew.mdo;

final class NativeRenderer {
    static {
        System.loadLibrary("mdo_cli");
    }

    private NativeRenderer() {}

    static native String renderMarkdown(
            String markdown, String fallbackTitle, long sourceModifiedUnixSeconds);
}
