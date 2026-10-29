package com.ultrastt.androidvoice;

import android.app.Activity;
import android.app.Instrumentation;
import android.os.Bundle;
import android.view.View;
import android.view.ViewGroup;
import android.widget.EditText;
import android.widget.TextView;

public final class ApiKeyPanelChecks extends Instrumentation {
    private int checks;

    @Override
    public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        start();
    }

    @Override
    public void onStart() {
        Bundle result = new Bundle();
        int status = Activity.RESULT_CANCELED;
        try {
            runOnMainSync(this::checkPanel);
            result.putString("stream", "PASS: " + checks + " saved-key UI checks using fake keys only; real credentials untouched\n");
            status = Activity.RESULT_OK;
        } catch (Exception | AssertionError failure) {
            result.putString("stream", "FAIL: " + failure.getClass().getSimpleName() + " " + failure.getMessage() + "\n");
        }
        finish(status, result);
    }

    private void checkPanel() {
        FakeStore store = new FakeStore();
        ApiKeyPanel panel = new ApiKeyPanel(getTargetContext(), store);
        require(text(panel, "✓ 저장 완료") != null && input(panel) == null, "Stored key shows locked summary without input");
        text(panel, "API 키 수정").performClick();
        require(input(panel) != null && input(panel).getText().length() == 0, "Modify opens an empty replacement field");
        input(panel).setText("unsaved-test-value");
        panel.refresh();
        require(store.saves == 0 && "old-test-value".equals(store.value), "Typing and refresh never overwrite stored key");
        text(panel, "수정 취소").performClick();
        require(input(panel) == null && "old-test-value".equals(store.value), "Cancel restores saved summary and old key");
        text(panel, "API 키 수정").performClick();
        text(panel, "API 키 저장").performClick();
        require(store.saves == 0 && input(panel) != null, "Empty replacement cannot erase old key");
        input(panel).setText("new-test-value");
        text(panel, "API 키 저장").performClick();
        require(store.saves == 1 && "new-test-value".equals(store.value), "Save explicitly commits new key once");
        require(input(panel) == null && text(panel, "✓ 저장 완료") != null, "Successful save closes editor and shows confirmation");
        panel = new ApiKeyPanel(getTargetContext(), store);
        require(input(panel) == null && text(panel, "API 키 수정") != null, "Reopening retains saved summary");
        text(panel, "API 키 수정").performClick();
        store.fail = true;
        input(panel).setText("failed-test-value");
        text(panel, "API 키 저장").performClick();
        require(input(panel) != null && "new-test-value".equals(store.value), "Failed write preserves old key and editor");
        require(text(panel, "API 키를 저장하지 못했습니다. 이전 키는 유지됩니다. 다시 시도해 주세요.") != null,
                "Failed write is clearly reported");
        store = new FakeStore();
        store.value = null;
        panel = new ApiKeyPanel(getTargetContext(), store);
        require(input(panel) != null && text(panel, "✓ 저장 완료") == null, "First use shows editor without false saved claim");
    }

    private EditText input(View view) {
        if (view.getVisibility() != View.VISIBLE) return null;
        if (view instanceof EditText) return (EditText) view;
        if (view instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) view;
            for (int index = 0; index < group.getChildCount(); index++) {
                EditText found = input(group.getChildAt(index));
                if (found != null) return found;
            }
        }
        return null;
    }

    private TextView text(View view, String expected) {
        if (view.getVisibility() != View.VISIBLE) return null;
        if (view instanceof TextView && expected.contentEquals(((TextView) view).getText())) return (TextView) view;
        if (view instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) view;
            for (int index = 0; index < group.getChildCount(); index++) {
                TextView found = text(group.getChildAt(index), expected);
                if (found != null) return found;
            }
        }
        return null;
    }

    private void require(boolean condition, String message) {
        checks++;
        if (!condition) throw new AssertionError(message);
    }

    private static final class FakeStore implements ApiKeyPanel.Store {
        String value = "old-test-value";
        int saves;
        boolean fail;
        public boolean hasKey() { return value != null; }
        public void save(String key) throws java.security.GeneralSecurityException {
            if (fail) throw new java.security.GeneralSecurityException("simulated write failure");
            saves++;
            value = key;
        }
        public void clear() { value = null; }
    }
}
