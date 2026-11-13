// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

package org.arca.viewer.ui;

import android.app.AlertDialog;
import android.content.ClipData;
import android.content.DialogInterface;
import android.content.Intent;
import android.net.Uri;
import android.widget.Toast;

import org.arca.viewer.MainActivity;
import org.arca.viewer.R;
import org.arca.viewer.core.Model;

/**
 * Sends a document to another app with the system share sheet. Arca keeps no copy for this: it
 * passes on the address it was given (the document the person opened, or the sterile copy they
 * saved) with a one-time read grant, so the receiving app reads the same bytes Arca checked.
 */
final class Share {
    private Share() {}

    /** Whether the report says the original can do something when it is opened elsewhere. */
    static boolean risky(Model.Report r) {
        return r != null && (r.verdict == Model.Verdict.ACTIVE_NEUTRALIZED || r.verdict == Model.Verdict.REJECTED);
    }

    /**
     * Shares the original. A document with active content gets a warning first: Arca blocked it
     * here, but the app that receives the file will not, so a sterile copy is offered instead.
     */
    static void original(final MainActivity act, final Uri uri, final String mime, Model.Report r,
                         final Runnable sterileCopy) {
        if (!risky(r)) {
            send(act, uri, mime, R.string.share_document);
            return;
        }
        AlertDialog.Builder b = new AlertDialog.Builder(act)
            .setTitle(R.string.share_risky_title)
            .setMessage(R.string.share_risky_message)
            .setNegativeButton(R.string.cancel, null)
            .setNeutralButton(R.string.share_risky_anyway, new DialogInterface.OnClickListener() {
                @Override
                public void onClick(DialogInterface d, int w) {
                    send(act, uri, mime, R.string.share_document);
                }
            });
        if (sterileCopy != null) {
            b.setPositiveButton(R.string.share_risky_sterile, new DialogInterface.OnClickListener() {
                @Override
                public void onClick(DialogInterface d, int w) {
                    sterileCopy.run();
                }
            });
        }
        Ui.showHardened(act, b);
    }

    /** Opens the share sheet for {@code uri}; says so when nothing can take it. */
    static void send(MainActivity act, Uri uri, String mime, int chooserTitle) {
        if (uri == null || !"content".equals(uri.getScheme())) {
            Toast.makeText(act, R.string.share_unavailable, Toast.LENGTH_LONG).show();
            return;
        }
        Intent send = new Intent(Intent.ACTION_SEND);
        send.setType(mime == null || mime.isEmpty() ? "application/octet-stream" : mime);
        send.putExtra(Intent.EXTRA_STREAM, uri);
        // The grant travels with the ClipData; a read grant only, and only for this one address.
        send.setClipData(ClipData.newRawUri("", uri));
        send.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        try {
            Intent chooser = Intent.createChooser(send, act.getString(chooserTitle));
            chooser.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            act.startActivity(chooser);
        } catch (RuntimeException e) {
            // No app takes it, or the provider does not let Arca pass the address on.
            Toast.makeText(act, R.string.share_unavailable, Toast.LENGTH_LONG).show();
        }
    }
}
