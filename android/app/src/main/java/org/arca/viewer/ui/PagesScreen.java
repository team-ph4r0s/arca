// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

package org.arca.viewer.ui;

import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.SeekBar;
import android.widget.TextView;

import org.arca.viewer.MainActivity;
import org.arca.viewer.Prefs;
import org.arca.viewer.R;
import org.arca.viewer.Session;
import org.arca.viewer.core.Capabilities;
import org.arca.viewer.core.Model;
import org.arca.viewer.core.PageDrawing;

import java.util.function.IntConsumer;

/**
 * The document drawn as it lays itself out, either as one long strip of pages to scroll through
 * or one page at a time. The drawings come from the sandbox as plain primitives (see
 * {@link PageView}); there is no decoding of pictures or fonts here.
 *
 * <p>The zoom is at the top; at the bottom are the slider over the pages, the way back to the
 * reading view, the switch for pictures and, in the right corner, the arrows to turn the page.
 */
public final class PagesScreen extends Screen {
    /** The page shown (or at the top of the strip); kept across rebuilds (rotation, folding). */
    private static int index;
    private static boolean started;
    /** One long strip of pages (the default) or one page at a time. */
    private static boolean continuous = true;
    /** Zoom of the sheet (1 = fits the width), kept across rebuilds; 25 % steps up to 400 %. */
    private static float sheetZoom = 1f;
    private static final float SHEET_ZOOM_STEP = 0.25f;
    private static final float SHEET_ZOOM_MAX = 4f;
    /** Pages on either side of the one in view that stay drawn in the strip (the rest are boxes). */
    private static final int KEEP_AROUND = 3;
    /** Sheet colours of a picture's placeholder box and its label. */
    private static final int BOX = 0xFFD5DAE0;
    private static final int BOX_TEXT = 0xFF46515E;

    /** The page controls, when the window is folded (see {@link #dock()}). */
    private View dockView;
    /** The strip of the continuous mode: one frame per page, and the scroller around it. */
    private FrameLayout[] slots;
    private boolean[] drawn;
    private ScrollView strip;
    private TextView position;
    private SeekBar slider;
    private View prevButton;
    private View nextButton;
    private int positionLabel;
    /** One page at a time: the drawing on screen and the page count it was built with. */
    private PageDrawing shown;
    private int shownPages;

    public PagesScreen(MainActivity act) {
        super(act);
    }

    @Override
    public View dock() {
        return dockView;
    }

    @Override
    public View build() {
        final Session s = Session.get();
        final Session.PageState st = s.page();
        Model.Report report = s.report();
        boolean slides = report != null && Capabilities.isPresentation(report.format);
        positionLabel = slides ? R.string.slides_position : R.string.pages_position;
        if (!started) {
            started = true;
            index = 0;
            sheetZoom = 1f;
        }
        if (st.pages == 0 && !st.running && s.pageFailure(0) == null) {
            later(new Runnable() {
                @Override
                public void run() {
                    Session.get().renderPage(0);
                }
            });
        }
        slots = null;
        drawn = null;
        strip = null;
        int pages = st.pages;
        if (pages > 0 && index >= pages) {
            index = pages - 1;
        }

        position = ui.text(pages == 0 ? "" : ui.str(positionLabel, index + 1, pages), Ui.SMALL);
        position.setGravity(Gravity.CENTER);

        LinearLayout body = ui.vbox();
        body.addView(topBar(slides), ui.fillWidth());
        View content = pages == 0 ? firstPage(s, st, slides)
            : continuous ? strip(s, pages) : single(s, st, slides);
        body.addView(content, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        if (pages > 0) {
            View bar = bottomBar(s, pages, slides);
            if (Hinge.hasDock()) {
                // Folded window (tabletop or book): the pages on one part, their controls on the other.
                dockView = dockPane(bar);
            } else {
                dockView = null;
                body.addView(bar, ui.fillWidth());
            }
        }
        return page(header(ui.str(slides ? R.string.slides_title : R.string.pages_title), true), body);
    }

    // ---- the bars ---------------------------------------------------------------------------

    /** At the top: one strip or one page at a time, and the zoom. */
    private View topBar(boolean slides) {
        LinearLayout row = ui.hbox();
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(ui.dp(8), 0, ui.dp(8), 0);
        row.addView(ui.button(continuous ? R.string.pages_mode_single : R.string.pages_mode_scroll, Ui.TEXT_ONLY,
            continuous ? R.drawable.ic_file : R.drawable.ic_list, new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    continuous = !continuous;
                    act.refresh();
                }
            }));
        row.addView(new View(act), new LinearLayout.LayoutParams(0, 1, 1f));
        row.addView(ui.iconButton(R.drawable.ic_minus, R.string.zoom_out, new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                onZoom(-1);
            }
        }));
        TextView zoom = ui.heading(ui.str(R.string.zoom_value, Math.round(sheetZoom * 100f)), Ui.LABEL);
        zoom.setGravity(Gravity.CENTER);
        row.addView(zoom, new LinearLayout.LayoutParams(ui.dp(56), ViewGroup.LayoutParams.WRAP_CONTENT));
        row.addView(ui.iconButton(R.drawable.ic_plus, R.string.zoom_in, new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                onZoom(1);
            }
        }));
        return row;
    }

    /**
     * At the bottom: the slider over the pages with the position, then the way back to the reading
     * view and the switch for pictures on the left, and the arrows to turn the page in the right
     * corner, where the thumb reaches them. Everything also works from a keyboard (PageUp /
     * PageDown, Ctrl + / Ctrl -).
     */
    private View bottomBar(final Session s, final int pages, boolean slides) {
        LinearLayout bar = ui.vbox();
        ui.bar(bar, true);
        bar.setPadding(ui.dp(12), ui.dp(6), ui.dp(12), ui.dp(8));

        LinearLayout top = ui.hbox();
        top.setGravity(Gravity.CENTER_VERTICAL);
        slider = new SeekBar(act);
        slider.setMax(Math.max(0, pages - 1));
        slider.setProgress(Math.min(index, Math.max(0, pages - 1)));
        slider.setEnabled(pages > 1);
        slider.setContentDescription(ui.str(slides ? R.string.slides_title : R.string.pages_slider));
        slider.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
            private boolean dragging;

            @Override
            public void onProgressChanged(SeekBar b, int progress, boolean fromUser) {
                if (fromUser) {
                    position.setText(ui.str(positionLabel, progress + 1, pages));
                    if (!dragging && progress != index) {
                        go(s, progress);
                    }
                }
            }

            @Override
            public void onStartTrackingTouch(SeekBar b) {
                dragging = true;
            }

            @Override
            public void onStopTrackingTouch(SeekBar b) {
                dragging = false;
                if (b.getProgress() != index) {
                    go(s, b.getProgress());
                }
            }
        });
        top.addView(slider, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        top.addView(ui.hspace(8));
        top.addView(position);
        bar.addView(top, ui.fillWidth());

        LinearLayout row = ui.hbox();
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.addView(ui.button(R.string.pages_bar_text, Ui.TONAL, 0, new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                act.back();
            }
        }));
        // Pictures on the page: on, they are decoded in the sandbox as soon as the page is drawn;
        // off, each is a grey box to tap. The same switch as in the reading view.
        final boolean pictures = Prefs.autoImages(act);
        row.addView(ui.iconButton(pictures ? R.drawable.ic_eye : R.drawable.ic_eye_off,
            pictures ? R.string.pictures_hide : R.string.pictures_show, new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    Prefs.setAutoImages(act, !pictures);
                    act.refresh();
                }
            }));
        row.addView(new View(act), new LinearLayout.LayoutParams(0, 1, 1f));
        prevButton = ui.iconButton(R.drawable.ic_chevron_left, slides ? R.string.slides_prev : R.string.pages_prev,
            new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    onPage(-1);
                }
            });
        nextButton = ui.iconButton(R.drawable.ic_chevron_right, slides ? R.string.slides_next : R.string.pages_next,
            new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    onPage(1);
                }
            });
        row.addView(prevButton);
        row.addView(ui.hspace(4));
        row.addView(nextButton);
        updateArrows(pages);
        bar.addView(row, ui.fillWidth());
        return bar;
    }

    private void updateArrows(int pages) {
        if (prevButton != null) {
            prevButton.setEnabled(index > 0);
            prevButton.setAlpha(index > 0 ? 1f : 0.4f);
        }
        if (nextButton != null) {
            nextButton.setEnabled(index + 1 < pages);
            nextButton.setAlpha(index + 1 < pages ? 1f : 0.4f);
        }
    }

    // ---- before the first page --------------------------------------------------------------

    private View firstPage(final Session s, Session.PageState st, boolean slides) {
        LinearLayout col = ui.vbox();
        col.setPadding(ui.dp(16), ui.dp(16), ui.dp(16), ui.dp(16));
        Session.Failure f = s.pageFailure(0);
        if (f != null) {
            col.addView(failure(s, 0, f, slides));
        } else {
            col.addView(ui.text(slides ? R.string.slides_loading : R.string.pages_loading, Ui.BODY));
        }
        return centered(col, 720);
    }

    private View failure(final Session s, final int page, Session.Failure f, boolean slides) {
        LinearLayout col = ui.vbox();
        col.addView(ui.text(Errors.text(f.status, s.isPicture()), Ui.BODY));
        col.addView(ui.text(slides ? R.string.slides_failed : R.string.pages_failed, Ui.SMALL));
        col.addView(ui.space(8));
        col.addView(ui.button(R.string.retry, Ui.TONAL, 0, new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                s.retryPage(page);
                act.refresh();
            }
        }));
        return col;
    }

    // ---- one page at a time -----------------------------------------------------------------

    private View single(final Session s, Session.PageState st, boolean slides) {
        LinearLayout col = ui.vbox();
        col.setPadding(ui.dp(12), ui.dp(8), ui.dp(12), ui.dp(16));
        PageDrawing d = s.cachedPage(index);
        shown = d;
        shownPages = st.pages;
        Session.Failure f = s.pageFailure(index);
        if (d == null && f != null) {
            col.addView(failure(s, index, f, slides));
        } else if (d == null) {
            col.addView(ui.text(slides ? R.string.slides_loading : R.string.pages_loading, Ui.BODY));
            final int wanted = index;
            later(new Runnable() {
                @Override
                public void run() {
                    Session.get().renderPage(wanted);
                }
            });
        } else {
            final int next = index + 1;
            later(new Runnable() {
                @Override
                public void run() {
                    // The next page is drawn ahead, so turning the page is quick.
                    Session.get().requestPages(new int[] {next});
                }
            });
            addNotes(col, d, s.report());
            PageView sheet = new PageView(act, d, BOX, BOX_TEXT);
            if (sheetZoom == 1f) {
                sheet.setSlideSwipe(new IntConsumer() {
                    @Override
                    public void accept(int delta) {
                        onPage(delta);
                    }
                });
            }
            col.addView(zoomed(sheet), ui.fillWidth());
        }
        ScrollView sv = new ScrollView(act);
        if (slides && sheetZoom == 1f) {
            sv.setFillViewport(true);
            col.setGravity(Gravity.CENTER_VERTICAL);
        }
        sv.addView(col);
        return centered(sv, 720);
    }

    private void addNotes(LinearLayout col, PageDrawing d, Model.Report report) {
        if (d.truncated) {
            col.addView(ui.text(R.string.pages_note_truncated, Ui.SMALL));
        }
        col.addView(ui.text(R.string.pages_note_approx, Ui.CAPTION));
        if (report != null && Capabilities.isVisio(report.format)) {
            col.addView(ui.text(R.string.pages_note_visio, Ui.CAPTION));
        }
        col.addView(ui.space(8));
    }

    /** Zoomed in, the sheet is wider than the window and scrolls sideways. */
    private View zoomed(View sheet) {
        if (sheetZoom <= 1f) {
            return sheet;
        }
        HorizontalScrollView sideways = new HorizontalScrollView(act);
        sideways.addView(sheet, new FrameLayout.LayoutParams(ui.dp(baseWidthDp() * sheetZoom),
            ViewGroup.LayoutParams.WRAP_CONTENT));
        return sideways;
    }

    private int baseWidthDp() {
        return Math.max(240, Math.min(Adaptive.widthDp(act), 720) - 24);
    }

    // ---- the strip of all pages -------------------------------------------------------------

    private View strip(final Session s, final int pages) {
        LinearLayout list = ui.vbox();
        list.setPadding(ui.dp(12), ui.dp(8), ui.dp(12), ui.dp(16));
        PageDrawing first = s.cachedPage(index);
        if (first == null) {
            first = s.cachedPage(0);
        }
        if (first != null) {
            addNotes(list, first, s.report());
        }
        slots = new FrameLayout[pages];
        drawn = new boolean[pages];
        for (int i = 0; i < pages; i++) {
            FrameLayout slot = new FrameLayout(act);
            LinearLayout.LayoutParams lp = ui.fillWidth();
            lp.bottomMargin = ui.dp(12);
            list.addView(slot, lp);
            slots[i] = slot;
            fillSlot(s, i);
        }
        strip = new ScrollView(act);
        strip.addView(zoomed(list));
        strip.setOnScrollChangeListener(new View.OnScrollChangeListener() {
            @Override
            public void onScrollChange(View v, int x, int y, int oldX, int oldY) {
                onStripScrolled(s, pages);
            }
        });
        // Back to where the reader was (a rebuild after rotation or a page drawn).
        final ScrollView sv = strip;
        sv.post(new Runnable() {
            @Override
            public void run() {
                if (strip == sv && slots != null && index < slots.length) {
                    sv.scrollTo(0, slotTop(index));
                    onStripScrolled(s, pages);
                }
            }
        });
        return centered(strip, sheetZoom > 1f ? 100_000 : 720);
    }

    /** Puts the drawn page in its frame, or a box of the page's size that says what is going on. */
    private void fillSlot(final Session s, final int i) {
        FrameLayout slot = slots[i];
        slot.removeAllViews();
        PageDrawing d = s.cachedPage(i);
        drawn[i] = d != null;
        if (d != null) {
            slot.addView(new PageView(act, d, BOX, BOX_TEXT), new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
            return;
        }
        Session.Failure f = s.pageFailure(i);
        TextView box = ui.text(f != null ? ui.str(Errors.text(f.status, s.isPicture()))
            : ui.str(R.string.pages_waiting, i + 1), Ui.SMALL);
        box.setGravity(Gravity.CENTER);
        box.setBackgroundColor(Theme.invertSheet(act) ? Theme.invertTone(0xFFFFFFFF) : 0xFFFFFFFF);
        box.setTextColor(BOX_TEXT);
        if (f != null) {
            box.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    s.retryPage(i);
                    fillSlot(s, i);
                }
            });
        }
        slot.addView(box, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, boxHeight(s, i)));
    }

    /** The height a page that is not drawn yet takes: that of a drawn neighbour at this width. */
    private int boxHeight(Session s, int i) {
        PageDrawing like = s.cachedPage(i);
        for (int k = 1; like == null && k < 4; k++) {
            like = s.cachedPage(i - k);
            if (like == null) {
                like = s.cachedPage(i + k);
            }
        }
        if (like == null) {
            like = s.page().drawing;
        }
        float ratio = like != null && like.width > 0f ? like.height / like.width : 1.414f;
        int width = ui.dp(baseWidthDp() * Math.max(1f, sheetZoom)) - ui.dp(24);
        return Math.max(ui.dp(120), Math.round(width * ratio));
    }

    private int slotTop(int i) {
        View slot = slots[i];
        View list = (View) slot.getParent();
        int top = slot.getTop();
        // The list is inside a sideways scroller when zoomed in.
        for (View v = list; v != null && v != strip; v = v.getParent() instanceof View ? (View) v.getParent() : null) {
            top += v.getTop();
        }
        return top;
    }

    /**
     * The page whose frame covers the upper third of the window is the current one; the pages in
     * view are drawn (the current first), and the far ones go back to boxes to save memory.
     */
    private void onStripScrolled(Session s, int pages) {
        if (strip == null || slots == null) {
            return;
        }
        int y = strip.getScrollY() + strip.getHeight() / 3;
        int current = index;
        for (int i = 0; i < slots.length; i++) {
            int top = slotTop(i);
            if (y >= top && y < top + slots[i].getHeight() + ui.dp(12)) {
                current = i;
                break;
            }
        }
        if (current != index) {
            index = current;
            position.setText(ui.str(positionLabel, index + 1, pages));
            if (slider != null) {
                slider.setProgress(index);
            }
            updateArrows(pages);
        }
        int bottom = strip.getScrollY() + strip.getHeight();
        java.util.List<Integer> want = new java.util.ArrayList<>();
        want.add(index);
        for (int i = index + 1; i < pages && slotTop(i) < bottom + strip.getHeight() / 2; i++) {
            want.add(i);
        }
        for (int i = index - 1; i >= 0 && i >= index - 1; i--) {
            want.add(i);
        }
        int[] order = new int[want.size()];
        for (int i = 0; i < order.length; i++) {
            order[i] = want.get(i);
        }
        s.requestPages(order);
        for (int i = 0; i < slots.length; i++) {
            boolean near = Math.abs(i - index) <= KEEP_AROUND || want.contains(i);
            if (drawn[i] && !near) {
                // Far from view: the page (and its pictures) is let go; its box keeps its height.
                View box = placeholder(i);
                drawn[i] = false;
                slots[i].removeAllViews();
                slots[i].addView(box);
            } else if (!drawn[i] && near && s.cachedPage(i) != null) {
                fillSlot(s, i);
            }
        }
    }

    private View placeholder(int i) {
        TextView box = ui.text(ui.str(R.string.pages_waiting, i + 1), Ui.SMALL);
        box.setGravity(Gravity.CENTER);
        box.setTextColor(BOX_TEXT);
        box.setLayoutParams(new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, slots[i].getHeight()));
        return box;
    }

    /** Runs {@code r} after the screen is built (asking for pages changes the session, which rebuilds). */
    private void later(Runnable r) {
        act.getWindow().getDecorView().post(r);
    }

    // ---- input ------------------------------------------------------------------------------

    @Override
    public boolean onZoom(int steps) {
        float next = Math.max(1f, Math.min(SHEET_ZOOM_MAX, sheetZoom + steps * SHEET_ZOOM_STEP));
        if (next != sheetZoom) {
            sheetZoom = next;
            act.refresh();
        }
        return true;
    }

    @Override
    public boolean onPage(int delta) {
        go(Session.get(), index + delta);
        return true;
    }

    /**
     * Shows page {@code to}. In the strip it scrolls there; one page at a time, it is drawn (or
     * taken from the cache). Asking while another page is being drawn is never lost.
     */
    private void go(Session s, int to) {
        int pages = s.page().pages;
        if (to < 0 || (pages > 0 && to >= pages)) {
            return;
        }
        index = to;
        if (continuous && strip != null && slots != null && to < slots.length) {
            strip.smoothScrollTo(0, slotTop(to));
            return;
        }
        act.refresh();
    }

    @Override
    public void onSessionChanged() {
        Session s = Session.get();
        int pages = s.page().pages;
        if (continuous && slots != null && slots.length == pages) {
            // A page arrived: put it in its frame without rebuilding the strip (which would lose
            // the scroll position and the pictures already shown).
            int i = s.page().index;
            if (i >= 0 && i < slots.length && !drawn[i] && (s.cachedPage(i) != null || s.pageFailure(i) != null)
                && Math.abs(i - index) <= KEEP_AROUND + 4) {
                fillSlot(s, i);
            }
            return;
        }
        if (!continuous && shown != null && shown == s.cachedPage(index) && shownPages == pages) {
            // Only a page drawn ahead arrived: the page in view stays as it is.
            return;
        }
        act.refresh();
    }

    @Override
    public void onLeave() {
        started = false;
        Session.get().closePage();
    }
}
