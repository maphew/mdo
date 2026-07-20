package io.github.maphew.mdo;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.res.Configuration;
import android.database.Cursor;
import android.graphics.Color;
import android.net.Uri;
import android.os.Bundle;
import android.os.Parcelable;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import android.view.Gravity;
import android.view.ViewGroup;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicInteger;

public final class MainActivity extends Activity {
    private static final int OPEN_DOCUMENT_REQUEST = 1;
    private static final int MAX_DOCUMENT_BYTES = 16 * 1024 * 1024;
    private static final String APP_ORIGIN = "https://mdo.invalid/";
    private static final String STATE_DOCUMENT_URI = "mdo:document-uri";

    private final AtomicInteger loadGeneration = new AtomicInteger();
    private TextView documentTitle;
    private WebView webView;
    private Uri currentUri;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        buildInterface();

        // setIntent() does not survive activity recreation, so a document
        // chosen through the picker is restored from instance state.
        Uri restored = savedInstanceState == null
                ? null : savedInstanceState.getParcelable(STATE_DOCUMENT_URI);
        if (restored != null) {
            loadDocument(restored);
            return;
        }
        if (!showIntent(getIntent())) {
            showWelcome();
        }
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        // No fallback here: a contentless relaunch (e.g. tapping the launcher
        // icon while the activity is on top) keeps the current document.
        showIntent(intent);
    }

    @Override
    protected void onSaveInstanceState(Bundle outState) {
        super.onSaveInstanceState(outState);
        if (currentUri != null) {
            outState.putParcelable(STATE_DOCUMENT_URI, currentUri);
        }
    }

    private boolean showIntent(Intent intent) {
        Uri uri = documentUri(intent);
        if (uri != null) {
            loadDocument(uri);
            return true;
        }
        String shared = sharedText(intent);
        if (shared != null) {
            loadSharedText(shared);
            return true;
        }
        return false;
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != OPEN_DOCUMENT_REQUEST || resultCode != RESULT_OK || data == null) {
            return;
        }

        Uri uri = data.getData();
        if (uri == null) {
            return;
        }
        if ((data.getFlags() & Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION) != 0) {
            try {
                getContentResolver().takePersistableUriPermission(
                        uri, Intent.FLAG_GRANT_READ_URI_PERMISSION);
            } catch (RuntimeException ignored) {
                // Some document providers grant access for this activity only.
            }
        }
        loadDocument(uri);
    }

    @Override
    protected void onDestroy() {
        loadGeneration.incrementAndGet();
        if (webView != null) {
            webView.stopLoading();
            webView.destroy();
        }
        super.onDestroy();
    }

    private void buildInterface() {
        boolean dark = (getResources().getConfiguration().uiMode
                & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        int background = dark ? Color.rgb(33, 33, 33) : Color.WHITE;
        int foreground = dark ? Color.rgb(220, 220, 220) : Color.rgb(33, 33, 33);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(background);
        // With targetSdk 36 Android 15+ enforces edge-to-edge; pad the layout
        // by the system-bar insets so the toolbar is not drawn under the
        // status bar and the WebView not under the gesture navigation bar.
        root.setFitsSystemWindows(true);

        LinearLayout toolbar = new LinearLayout(this);
        toolbar.setOrientation(LinearLayout.HORIZONTAL);
        toolbar.setGravity(Gravity.CENTER_VERTICAL);
        int padding = dp(8);
        toolbar.setPadding(padding, padding, padding, padding);

        Button open = new Button(this);
        open.setText(R.string.open_document);
        open.setOnClickListener(view -> openDocumentPicker());
        toolbar.addView(open, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT));

        documentTitle = new TextView(this);
        documentTitle.setTextColor(foreground);
        documentTitle.setTextSize(18);
        documentTitle.setSingleLine(true);
        documentTitle.setPadding(dp(12), 0, 0, 0);
        toolbar.addView(documentTitle, new LinearLayout.LayoutParams(
                0, ViewGroup.LayoutParams.WRAP_CONTENT, 1));
        root.addView(toolbar, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT));

        webView = new WebView(this);
        configureWebView(webView);
        root.addView(webView, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1));
        setContentView(root);
    }

    private void configureWebView(WebView view) {
        WebSettings settings = view.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);
        settings.setBlockNetworkLoads(true);
        settings.setJavaScriptCanOpenWindowsAutomatically(false);
        settings.setSupportMultipleWindows(true);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);

        view.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView ignored, WebResourceRequest request) {
                return handleLink(request.getUrl());
            }

            @Override
            @SuppressWarnings("deprecation")
            public boolean shouldOverrideUrlLoading(WebView ignored, String url) {
                return handleLink(Uri.parse(url));
            }
        });
    }

    private boolean handleLink(Uri uri) {
        if ("mdo.invalid".equals(uri.getHost()) && uri.getFragment() != null) {
            return false;
        }
        try {
            startActivity(new Intent(Intent.ACTION_VIEW, uri));
        } catch (ActivityNotFoundException error) {
            Toast.makeText(this, R.string.no_link_handler, Toast.LENGTH_SHORT).show();
        }
        return true;
    }

    private void openDocumentPicker() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT)
                .addCategory(Intent.CATEGORY_OPENABLE)
                .setType("text/*")
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                        | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
        try {
            startActivityForResult(intent, OPEN_DOCUMENT_REQUEST);
        } catch (ActivityNotFoundException error) {
            Toast.makeText(this, R.string.no_document_picker, Toast.LENGTH_LONG).show();
        }
    }

    private void loadDocument(Uri uri) {
        currentUri = uri;
        int generation = loadGeneration.incrementAndGet();
        documentTitle.setText(getString(
                R.string.loading_document, getString(R.string.untitled_document)));

        new Thread(() -> {
            try {
                // Provider queries stay off the main thread: cloud document
                // providers can block for seconds and trigger an ANR.
                String title = queryDisplayName(uri);
                String markdown = readDocument(uri);
                long modifiedSeconds = queryModifiedSeconds(uri);
                String html = NativeRenderer.renderMarkdown(
                        markdown, stripExtension(title), modifiedSeconds);
                showRendered(generation, title, html);
            } catch (Exception | UnsatisfiedLinkError error) {
                runOnUiThread(() -> showError(generation, error));
            }
        }, "mdo-render").start();
    }

    private void loadSharedText(String markdown) {
        currentUri = null;
        int generation = loadGeneration.incrementAndGet();
        String title = getString(R.string.shared_text);
        documentTitle.setText(getString(R.string.loading_document, title));

        new Thread(() -> {
            try {
                String html = NativeRenderer.renderMarkdown(markdown, title, -1);
                showRendered(generation, title, html);
            } catch (Exception | UnsatisfiedLinkError error) {
                runOnUiThread(() -> showError(generation, error));
            }
        }, "mdo-render").start();
    }

    private void showRendered(int generation, String title, String html) {
        runOnUiThread(() -> {
            if (generation != loadGeneration.get() || isDestroyed()) {
                return;
            }
            documentTitle.setText(title);
            webView.loadDataWithBaseURL(APP_ORIGIN, html, "text/html", "UTF-8", APP_ORIGIN);
        });
    }

    private void showWelcome() {
        currentUri = null;
        int generation = loadGeneration.incrementAndGet();
        try {
            String markdown = getString(R.string.welcome_markdown);
            String html = NativeRenderer.renderMarkdown(markdown, getString(R.string.app_name), -1);
            documentTitle.setText(R.string.app_name);
            webView.loadDataWithBaseURL(APP_ORIGIN, html, "text/html", "UTF-8", APP_ORIGIN);
        } catch (RuntimeException | UnsatisfiedLinkError error) {
            // A missing or incompatible native library (e.g. a non-ARM64
            // device) must not crash the app on first launch.
            showError(generation, error);
        }
    }

    private void showError(int generation, Throwable error) {
        if (generation != loadGeneration.get() || isDestroyed()) {
            return;
        }
        documentTitle.setText(R.string.could_not_open);
        Toast.makeText(this, getString(R.string.open_error, error.getMessage()), Toast.LENGTH_LONG)
                .show();
    }

    private Uri documentUri(Intent intent) {
        if (intent == null) {
            return null;
        }
        if (Intent.ACTION_VIEW.equals(intent.getAction())) {
            return intent.getData();
        }
        if (Intent.ACTION_SEND.equals(intent.getAction())) {
            // MainActivity is exported, so EXTRA_STREAM may hold any
            // Parcelable a sender chooses; an unchecked cast would let other
            // apps crash us.
            @SuppressWarnings("deprecation")
            Parcelable shared = intent.getParcelableExtra(Intent.EXTRA_STREAM);
            return shared instanceof Uri ? (Uri) shared : null;
        }
        return null;
    }

    private String sharedText(Intent intent) {
        if (intent == null || !Intent.ACTION_SEND.equals(intent.getAction())) {
            return null;
        }
        CharSequence text = intent.getCharSequenceExtra(Intent.EXTRA_TEXT);
        return text == null || text.length() == 0 ? null : text.toString();
    }

    // Match the desktop CLI's fallback-title contract, which passes the file
    // stem ("notes"), not the display name ("notes.md").
    private static String stripExtension(String name) {
        int dot = name.lastIndexOf('.');
        return dot > 0 ? name.substring(0, dot) : name;
    }

    private String readDocument(Uri uri) throws IOException {
        try (InputStream input = getContentResolver().openInputStream(uri)) {
            if (input == null) {
                throw new IOException("The document provider returned no data");
            }
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) {
                if (bytes.size() + count > MAX_DOCUMENT_BYTES) {
                    throw new IOException("Document is larger than 16 MB");
                }
                bytes.write(buffer, 0, count);
            }
            String text = new String(bytes.toByteArray(), StandardCharsets.UTF_8);
            return text.startsWith("\uFEFF") ? text.substring(1) : text;
        }
    }

    private String queryDisplayName(Uri uri) {
        try (Cursor cursor = getContentResolver().query(
                uri, new String[]{OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst() && !cursor.isNull(0)) {
                return cursor.getString(0);
            }
        } catch (RuntimeException ignored) {
            // Fall back to the last URI segment for unusual providers.
        }
        String segment = uri.getLastPathSegment();
        return segment == null || segment.isEmpty() ? getString(R.string.untitled_document) : segment;
    }

    private long queryModifiedSeconds(Uri uri) {
        try (Cursor cursor = getContentResolver().query(
                uri,
                new String[]{DocumentsContract.Document.COLUMN_LAST_MODIFIED},
                null,
                null,
                null)) {
            if (cursor != null && cursor.moveToFirst() && !cursor.isNull(0)) {
                long milliseconds = cursor.getLong(0);
                // Providers report 0 for "unknown" (File.lastModified()
                // convention); treat it as missing, not the 1970 epoch.
                return milliseconds > 0 ? milliseconds / 1000 : -1;
            }
        } catch (RuntimeException ignored) {
            // Modified time is optional and must never block rendering.
        }
        return -1;
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
