package com.ultrastt.androidvoice;

import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

final class RecoveryView extends LinearLayout {
    private final TextView text;
    private final TextView feedback;
    private final Runnable onDiscard;
    private RecoveryResult.Snapshot shown;

    RecoveryView(Context context, Runnable onDiscard) {
        super(context);
        this.onDiscard = onDiscard;
        setOrientation(VERTICAL);
        int padding = Ui.dp(context, 16);
        setPadding(padding, padding, padding, padding);
        setBackgroundColor(0xFF174639);
        addView(Ui.text(context, "입력하지 못한 문장", 18, true));
        addView(Ui.text(context, "입력창이 바뀌었거나 자동 입력에 실패했습니다. 문장을 복사해 원하는 곳에 붙여넣으세요.", 13, false));
        text = Ui.text(context, "", 15, false);
        text.setPadding(0, Ui.dp(context, 12), 0, Ui.dp(context, 12));
        addView(text, Ui.block(context));
        feedback = Ui.text(context, "", 13, false);
        feedback.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        addView(feedback);
        Button copy = Ui.button(context, "문장 복사");
        copy.setOnClickListener(view -> copy());
        addView(copy, Ui.block(context));
        Button discard = Ui.button(context, "닫기 · 문장 지우기");
        discard.setOnClickListener(view -> {
            if (shown != null) {
                RecoveryResult.ACTIVE.clear(shown.id);
                shown = null;
            }
            setVisibility(GONE);
            if (onDiscard != null) {
                onDiscard.run();
            }
        });
        addView(discard, Ui.block(context));
        setVisibility(GONE);
    }

    void show(RecoveryResult.Snapshot snapshot) {
        shown = snapshot;
        if (snapshot == null) {
            setVisibility(GONE);
            return;
        }
        text.setText(snapshot.text);
        feedback.setText("");
        setVisibility(VISIBLE);
    }

    private void copy() {
        RecoveryResult.Snapshot current = RecoveryResult.ACTIVE.latest();
        if (shown == null || current == null || current.id != shown.id) {
            feedback.setText("문장이 더 이상 남아 있지 않습니다.");
            return;
        }
        try {
            ClipboardManager clipboard = (ClipboardManager) getContext().getSystemService(Context.CLIPBOARD_SERVICE);
            if (clipboard == null) {
                throw new IllegalStateException("Clipboard unavailable");
            }
            clipboard.setPrimaryClip(ClipData.newPlainText("Ultra STT 문장", shown.text));
            feedback.setText("복사했습니다. 원하는 입력창에 붙여넣으세요.");
        } catch (RuntimeException error) {
            feedback.setText("복사하지 못했습니다. 잠시 후 다시 시도해 주세요.");
        }
    }
}
