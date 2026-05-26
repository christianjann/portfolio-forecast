package dev.gpui.mobile;

import android.app.Activity;
import android.content.ContentResolver;
import android.net.Uri;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;

/**
 * Helper for reading the full contents of a file identified by a content URI.
 *
 * <p>Android's Storage Access Framework returns {@code content://} URIs (not
 * file-system paths) when the user picks a file with {@link GpuiFilePicker}.
 * {@code std::fs::File::open()} cannot open a content URI, so this class bridges
 * the gap: Rust calls {@link #readAllBytes} and passes the returned byte array
 * directly to {@code pp_reader::load_from_bytes()}.</p>
 *
 * <p>Called from Rust via JNI — see {@code src/android_content_reader.rs}.</p>
 */
public final class GpuiContentReader {

    private GpuiContentReader() {}

    /**
     * Read the full content of the file identified by {@code uriString}.
     *
     * @param activity  The current Activity (needed to reach {@link ContentResolver}).
     * @param uriString The URI returned by {@link GpuiFilePicker#openFile}
     *                  (e.g. {@code content://com.android.providers.…/…}).
     * @return The file bytes, or {@code null} if the URI could not be opened.
     */
    public static byte[] readAllBytes(final Activity activity, final String uriString) {
        if (uriString == null || uriString.isEmpty()) {
            return null;
        }
        try {
            Uri uri = Uri.parse(uriString);
            ContentResolver cr = activity.getContentResolver();
            try (InputStream is = cr.openInputStream(uri)) {
                if (is == null) return null;
                ByteArrayOutputStream buf = new ByteArrayOutputStream();
                byte[] chunk = new byte[65536];
                int n;
                while ((n = is.read(chunk)) != -1) {
                    buf.write(chunk, 0, n);
                }
                return buf.toByteArray();
            }
        } catch (Exception e) {
            android.util.Log.e("GpuiContentReader", "readAllBytes failed for " + uriString, e);
            return null;
        }
    }
}
